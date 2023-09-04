use super::settings;
use log::{debug, info, trace};
use socket2::{Domain, Type};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Write};
use std::net::{IpAddr, Ipv4Addr};

pub(crate) fn connect_to_editor(
    settings: &settings::Settings,
) -> Result<socket2::Socket, std::io::Error> {
    let socket = socket2::Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();

    debug!("Host: {}", settings.host.as_ref().unwrap());
    let addr_srv = if settings.host.as_ref().unwrap() == "localhost" {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    } else {
        settings
            .host
            .as_ref()
            .unwrap()
            .parse::<IpAddr>()
            .map_err(|e| Error::new(ErrorKind::AddrNotAvailable, e.to_string()))?
    };
    let addr_srv = std::net::SocketAddr::new(addr_srv, settings.port.unwrap()).into();

    debug!("About to connect to {:?}", addr_srv);
    socket.connect(&addr_srv)?;
    trace!(
        "Socket details: \n\tmy address: {:?}\n\tremote address {:?}",
        socket.local_addr()?,
        socket.peer_addr()?
    );
    Ok(socket)
}

pub(crate) fn close_buffer(
    opened_buffers: &mut HashMap<String, settings::OpenedBuffer>,
    buffer_reader: &mut BufReader<&socket2::Socket>,
) {
    let mut myline = String::with_capacity(128);

    while let Ok(n) = buffer_reader.read_line(&mut myline) {
        if n == 0 || myline.trim() == "" {
            trace!("Finished receiving closing instructions");
            break;
        }
        let command: Vec<&str> = myline.trim().splitn(2, ':').collect::<Vec<&str>>();
        trace!("  close instruction:\t{:?}", command);
        let (_, closed_buffer) = opened_buffers.remove_entry(command[1].trim()).unwrap();
        info!("Closed: {:?}", closed_buffer.canon_path.as_os_str());
        myline.clear();
    }
}

pub(crate) fn open_file_in_remote(
    socket: &socket2::Socket,
    buffers: &HashMap<String, settings::OpenedBuffer>,
) -> Result<(), String> {
    // ) -> Result<HashMap<String, settings::OpenedBuffer>, String> {
    let bsize = socket.recv_buffer_size().map_err(|e| e.to_string())?;
    trace!("Socket recv buffer: {}", bsize);
    let bsize = socket.send_buffer_size().map_err(|e| e.to_string())? * 2;
    trace!("Socket send buffer: {}", bsize);

    {
        let mut buf_writer = BufWriter::with_capacity(bsize, socket);
        for (token, opened_buffer) in buffers {
            // For each buffer get the header values:
            // - display-name
            // - real-path
            // - selection/line
            // - file-type (optional)
            let mut total = 0usize;
            let header = format!(
                concat!(
                    "open\ndisplay-name: {}\n",
                    "real-path: {}\n",
                    "selection: {}\n",
                    "data-on-save: yes\nre-activate: yes\n",
                    "token: {}\n",
                ),
                // host_name.to_string_lossy(),
                opened_buffer.display_name.to_string_lossy(),
                opened_buffer.canon_path.to_string_lossy(),
                opened_buffer.line,
                token
            );
            trace!("header: {}", header);
            buf_writer
                .write(header.as_bytes())
                .map_err(|e| e.to_string())?;
            buf_writer.flush().map_err(|e| e.to_string())?;

            if let Some(filetype) = &opened_buffer.filetype {
                writeln!(&mut buf_writer, "file-type: {filetype}").map_err(|e| e.to_string())?;
                debug!("file-type: {}", filetype);
            }
            writeln!(&mut buf_writer, "data: {}", opened_buffer.size).map_err(|e| e.to_string())?;
            buf_writer.flush().map_err(|e| e.to_string())?;

            // Read file from disk and send it over the socket
            let fp = File::open(&opened_buffer.canon_path).map_err(|e| e.to_string())?;
            let mut buf_reader = BufReader::with_capacity(bsize, fp);
            debug!(
                "-> Opening {} (size: {} bytes).",
                opened_buffer.canon_path.to_string_lossy(),
                opened_buffer.size,
            );
            loop {
                let buffer = buf_reader.fill_buf().map_err(|e| e.to_string())?;
                let length = buffer.len();
                if length == 0 {
                    debug!("  no more data could be read");
                    break;
                }
                total += length;
                buf_writer.write_all(buffer).map_err(|e| e.to_string())?;
                buf_writer.flush().map_err(|e| e.to_string())?;
                trace!("  sent a chunk of {} bytes to remote editor", length);
                buf_reader.consume(length);
            }
            // Signal we are done sending this file
            let _n = buf_writer
                .write_fmt(format_args!("\n.\n"))
                .map_err(|e| e.to_string());
            buf_writer.flush().map_err(|e| e.to_string())?;
            debug!("  os-reported file size: {}", opened_buffer.size);
            debug!("  actual bytes read: {}", total);
            info!(
                "<- Finished opening {:?} in remote.",
                opened_buffer.canon_path
            );
        }
    }

    let mut b = [0u8; 512];
    debug!("Waiting for remote editor to identify itself...");
    let n = socket.recv(&mut b).map_err(|e| e.to_string())?;
    assert!(n < 512 && n != 0);
    debug!(
        "Connected to remote app: {}",
        String::from_utf8_lossy(&b[0..n]).trim()
    );
    // Ok(buffers)
    Ok(())
}
