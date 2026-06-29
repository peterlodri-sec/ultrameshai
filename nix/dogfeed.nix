{ pkgs }:

let
  bunWithTypes = pkgs.bun.overrideAttrs (old: {
    buildInputs = (old.buildInputs or []) ++ [ pkgs.bunPackages.bun-types ];
  });
in
pkgs.mkShell {
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
    echo "🐕 dogfeed shell — self-improving data generation loop"
    echo "  bun src/index.ts          — run the loop"
    echo "  bun test                  — run tests"
    echo "  bun examples/basic-loop.ts — run example"
    echo ""
    echo "  Set OPENROUTER_KEY and HF_TOKEN to enable LLM calls"
  '';
}
