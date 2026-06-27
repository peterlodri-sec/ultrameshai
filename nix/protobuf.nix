{ pkgs }:

pkgs.stdenv.mkDerivation {
  name = "loop-engineering-protobuf-gen";
  src = ../proto;

  nativeBuildInputs = with pkgs; [
    protobuf
    protoc-gen-prost
  ];

  buildPhase = ''
    mkdir -p $out/rust
    protoc \
      --prost_out=$out/rust \
      --prost_opt=compile=false \
      loop_engineering.proto
  '';

  installPhase = ''
    cp -r $out/rust $out/
  '';
}