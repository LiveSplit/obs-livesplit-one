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

    cp target/$TARGET/release/obs_livesplit_one.dll $stage/obs-livesplit-one.dll 2>/dev/null || :
    cp target/$TARGET/release/libobs_livesplit_one.so $stage/libobs_livesplit_one.so 2>/dev/null || :
    cp target/$TARGET/release/libobs_livesplit_one.dylib $stage/libobs_livesplit_one.dylib 2>/dev/null || :

    cd $stage
    if [ "$OS_NAME" = "windows-latest" ]; then
        7z a $src/obs-livesplit-one-$tag-$TARGET.zip *
    else
        tar czf $src/obs-livesplit-one-$tag-$TARGET.tar.gz *
    fi
    cd $src

    rm -rf $stage
}

main
