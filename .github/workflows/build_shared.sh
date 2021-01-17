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

    if [ "$OS_NAME" = "macOS-latest" ]; then
        curl https://cdn-fastly.obsproject.com/downloads/obs-mac-26.1.2.dmg -o obs.dmg
        hdiutil attach obs.dmg
        mkdir libobs
        cp "/Volumes/OBS-Studio 26.1.2/OBS.app/Contents/Frameworks/libobs.0.dylib" "./libobs/libobs.dylib"
        export RUSTFLAGS="-L libobs"
    fi

    $cargo build --target $TARGET $release_flag $FEATURES
}

main
