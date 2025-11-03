{
  description = "dev-env for midnight miner";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.cargo-watch
            pkgs.pkg-config
            pkgs.openssl
            pkgs.libsodium
            pkgs.git
            pkgs.protobuf
          ];

          shellHook = ''
            echo "midnight mine dev environment loaded"
            echo "Rust: $(rustc --version)"
          '';
        };
      }
    );
}