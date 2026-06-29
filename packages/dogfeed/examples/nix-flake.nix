# nix-flake.nix — standalone flake for self-hosting dogfeed
#
# Usage:
#   nix develop                    # enter dev shell
#   OPENROUTER_KEY=sk-... nix run  # run the loop
#
# To use as a flake input in another project:
#   inputs.dogfeed.url = "github:peterlodri-sec/ultrameshai/packages/dogfeed";
#
# Then in your flake outputs:
#   dogfeed.packages.${system}.default
{
  description = "dogfeed — self-improving data generation loop";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ bun nodejs_22 sqlite jq curl git ];
          shellHook = ''
            echo "🐕 dogfeed shell — bun src/index.ts to start"
          '';
        };

        packages.default = pkgs.stdenv.mkDerivation {
          pname = "dogfeed";
          version = "0.1.0";
          src = ./.;
          buildInputs = [ pkgs.bun ];
          buildPhase = "bun build src/index.ts --outdir $out --target bun";
          installPhase = "true";
        };
      });
}
