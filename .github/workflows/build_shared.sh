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

    if [ -z "$FEATURES" ]; then
        FEATURE_FLAGS="--no-default-features"
    else
        FEATURE_FLAGS="--no-default-features --features $FEATURES"
    fi

    if [ "$OS_NAME" = "windows-latest" ]; then
        $cargo build -p obs --target $TARGET $release_flag $FEATURE_FLAGS
        cp target/$TARGET/$target_folder/obs.dll.lib ./obs.lib
    fi

    $cargo build --target $TARGET $release_flag $FEATURE_FLAGS

    if [ "$OS_NAME" = "macOS-latest" ]; then
        install_name_tool -change $(pwd)/target/$TARGET/$target_folder/deps/libobs.dylib @rpath/libobs.0.dylib target/$TARGET/$target_folder/libobs_livesplit_one.dylib
    fi
}

main
