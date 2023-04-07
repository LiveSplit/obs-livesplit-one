set -ex

main() {
    local tag=$(git tag --points-at HEAD)
    local src=$(pwd) \
          stage=

    if [ "$OS_NAME" = "macOS-latest" ]; then
        stage=$(mktemp -d -t tmp)
    else
        stage=$(mktemp -d)
    fi

    if [ "$OS_NAME" = "ubuntu-latest" ]; then
        mkdir -p $stage/obs-livesplit-one/bin/$PLUGIN_BITS
        cp target/$TARGET/max-opt/libobs_livesplit_one.so $stage/obs-livesplit-one/bin/$PLUGIN_BITS/libobs-livesplit-one.so
    elif [ "$OS_NAME" = "macOS-latest" ]; then
        cp target/$TARGET/max-opt/libobs_livesplit_one.dylib $stage/obs-livesplit-one.so
    elif [ "$OS_NAME" = "windows-latest" ]; then
        cp target/$TARGET/max-opt/obs_livesplit_one.dll $stage/obs-livesplit-one.dll
    fi

    cd $stage
    if [ "$OS_NAME" = "windows-latest" ]; then
        7z a $src/obs-livesplit-one-$tag-$RELEASE_TARGET.zip *
    else
        tar czf $src/obs-livesplit-one-$tag-$RELEASE_TARGET.tar.gz *
    fi
    cd $src

    rm -rf $stage
}

main
