set -ex

main() {
    local cargo=cross
    if [ "$SKIP_CROSS" = "skip" ]; then
        cargo=cargo
    fi
    local release_flag=""
    if [ "$IS_DEPLOY" = "true" ]; then
        release_flag="--release"
    fi

    if [ "$OS_NAME" = "ubuntu-latest" ]; then
        sudo add-apt-repository ppa:obsproject/obs-studio -y
        sudo apt update
        sudo apt install obs-studio -y
    fi

    $cargo build --target $TARGET $release_flag $FEATURES
}

main
