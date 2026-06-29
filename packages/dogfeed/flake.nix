{
  description = "dogfeed — self-improving data generation loop for LLM training";

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
          name = "dogfeed";

          packages = with pkgs; [
            bun
            nodejs_22
            sqlite
            jq
            curl
            git
          ];

          shellHook = ''
            echo "🐕 dogfeed — self-improving data generation loop"
            echo ""
            echo "Quick start:"
            echo "  bun install"
            echo "  OPENROUTER_KEY=sk-... bun src/index.ts"
            echo ""
            echo "Commands:"
            echo "  bun test                  — run tests (48 tests)"
            echo "  bun src/index.ts          — run the loop"
            echo "  bun examples/basic-loop.ts — run basic example"
            echo "  bun run build             — build dist/"
            echo ""
            echo "Config (env vars):"
            echo "  OPENROUTER_KEY   — OpenRouter API key (free tier works)"
            echo "  HF_TOKEN         — HuggingFace token for publishing"
            echo "  HF_REPO          — HuggingFace dataset repo (org/name)"
            echo "  TOPICS           — Comma-separated topic list"
            echo "  COMPRESS=true    — Enable kompress-ultra compression"
          '';
        };

        packages.default = pkgs.stdenv.mkDerivation {
          pname = "dogfeed";
          version = "0.1.0";
          src = ./.;
          buildInputs = [ pkgs.bun ];
          buildPhase = ''
            bun build src/index.ts --outdir $out --target bun
          '';
          installPhase = "true";
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/index.js";
        };
      });
}
