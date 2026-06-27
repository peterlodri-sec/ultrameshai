{
  description = "Loop-engineering agent stack substrate";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        agentUnit = import ./nix/agent-unit.nix { inherit pkgs; };
      in
      {
        devShells = {
          default = agentUnit.standard;
          agent-unit = agentUnit.standard;
          agent-unit-test = agentUnit.test;
          agent-unit-red-team = agentUnit.redTeam;
          agent-unit-devops = agentUnit.devops;
        };

        packages.protobuf-gen = import ./nix/protobuf.nix { inherit pkgs; };
      });
}