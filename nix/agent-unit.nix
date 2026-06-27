{ pkgs }:

let
  basePackages = with pkgs; [
    nushell
    protobuf
    protoc-gen-prost  # Rust protobuf
    git
    curl
    jq
  ];

  mkShell = extraPackages: pkgs.mkShell {
    packages = basePackages ++ extraPackages;
  };
in
{
  standard = mkShell (with pkgs; [
    rustc
    cargo
    rust-analyzer
  ]);

  test = mkShell (with pkgs; [
    rustc
    cargo
    cargo-nextest
    rust-analyzer
  ]);

  redTeam = mkShell (with pkgs; [
    bpftrace
    libbpf
    elfutils
    # CAP_NET_ADMIN must be granted at runtime, not in nix shell
  ]);

  devops = mkShell (with pkgs; [
    git
    nix-prefetch
    nixpkgs-review
  ]);
}