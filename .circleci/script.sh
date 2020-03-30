# This script takes care of testing your crate

set -ex

run_tests() {
    runner="$1"
    working_dir="$2"
    # runner="cargo run --target "$TARGET" --"
    export RMATE_HOST=localhost
    export RMATE_PORT=55555
    case "$TARGET" in
        x86_64-apple-darwin)
            $runner --help
            ;;
        x86_64-unknown-linux-gnu)
            $runner --help
            ;;
        i686-unknown-linux-gnu)
            $runner --help
            ;;
        i686-apple-darwin)
            $runner --help
            ;;
        x86_64-unknown-freebsd)
            $runner --help
            # $runner config --authorization hamid:12345
            # $runner config -d
            ;;
        armv7-linux-androideabi)
            $runner --help
            ;;
        *)
            return
            ;;
    esac
}

# if [[ "$TARGET" == "i686-unknown-linux-gnu" ]]; then
#   source /home/circleci/.cargo/env
# fi

cargo generate-lockfile

# Build only
if [[ -z "$CIRCLE_TEST" || "$CIRCLE_TEST" == 'false' ]]; then
    echo "Tests Disabled. Just Building in $BUILD_TYPE mode"
    arg=
    [[ "$BUILD_TYPE" == "release" ]] && arg="--release"
    cargo build $arg --target "$TARGET"
# Test only
elif [[ "$CIRCLE_TEST" == "true" ]]; then
    echo "$1"
    run_tests "$1" "$2"
else 
    echo "CIRCLE_TEST env. variable has to be either false or true: $CIRCLE_TEST"
fi
