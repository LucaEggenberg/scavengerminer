{
  description = "miner for the midnighttoken scavenger hunt";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ]
    (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };

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
            echo "Midnight Miner Dev Environment Loaded"
            echo "Rust: $(rustc --version)"
          '';
        };

        packages.miner = pkgs.rustPlatform.buildRustPackage {
          pname = "scavenger-miner";
          version = "0.1.0";

          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;

            outputHashes = {
              "ashmaize-0.1.0" = "sha256-4l8vfkA7Ri59uhfyyV0IQ+/T5PsMKTcQpuOIvVbeEjA=";
            };
          };

          buildInputs = [
            pkgs.openssl
            pkgs.libsodium
          ];

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.protobuf
          ];

          doCheck = false;
        };
      }
    );
}