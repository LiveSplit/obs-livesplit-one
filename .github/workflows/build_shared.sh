set -ex

main() {
    local cargo=cross
    if [ "$SKIP_CROSS" = "skip" ]; then
        cargo=cargo
    fi
    local release_flag=""
    local target_folder="debug"
    if [ "$IS_DEPLOY" = "true" ]; then
        release_flag="--release"
        target_folder="release"
    fi

    $cargo build -p obs --target $TARGET $release_flag $FEATURES

    if [ "$OS_NAME" = "macOS-latest" ]; then
        cp target/$TARGET/$target_folder/libobs.dylib .
    elif [ "$OS_NAME" = "windows-latest" ]; then
        cp target/$TARGET/$target_folder/obs.dll.lib ./obs.lib
    fi

    $cargo build --target $TARGET $release_flag $FEATURES
}

main
