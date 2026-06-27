# Placeholder — real derivation added in Task 2
{ pkgs }:
pkgs.stdenv.mkDerivation {
  name = "loop-engineering-protobuf-gen-stub";
  src = ../proto;
  nativeBuildInputs = with pkgs; [ protobuf ];
  buildPhase = "mkdir -p $out";
  installPhase = "mkdir -p $out/rust";
}