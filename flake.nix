{
  description = "Cloudflare CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cloudflare";
          version = "0.1.0";
          src = self;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          meta = {
            mainProgram = "cloudflare";
            description = "Cloudflare CLI";
            homepage = "https://github.com/radjathaher/cloudflare-cli";
            license = pkgs.lib.licenses.mit;
          };
        };
      }
    );
}
