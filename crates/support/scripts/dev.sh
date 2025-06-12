# dev
PKG=ghcr.io/gnosisguild/e3-support:next

docker run -it \
  -v $(pwd)/app:/app/app \
  -v $(pwd)/host:/app/host \
  -v $(pwd)/methods:/app/methods \
  -v $(pwd)/program:/app/program \
  -v $(pwd)/scripts:/app/scripts \
  -v $(pwd)/contracts:/app/contracts \
  -v $(pwd)/tests:/app/tests \
  -v $(pwd)/Cargo.toml:/app/Cargo.toml \
  -v $(pwd)/Cargo.lock:/app/Cargo.lock \
  $PKG
