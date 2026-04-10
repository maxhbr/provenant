{
  description = "Provenant – Rust-based ScanCode-compatible scanner for licenses, package metadata, SBOMs, and provenance data";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};

        # Pin the Rust toolchain to the version declared in rust-toolchain.toml
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-zC8E38iDVJ1oPIzCqTk/Ujo9+9kx9dXq7wAwPMpkpg0=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustToolchain);

        # Filter source to only include Rust-relevant files
        src = let
          # Keep non-Rust files that the build needs
          extraFilter = path: _type:
            (builtins.match ".*\\.zst$" path) != null # license index
            || (baseNameOf path) == "NOTICE"; # included via include_str! in main.rs
          combinedFilter = path: type:
            (craneLib.filterCargoSources path type) || (extraFilter path type);
        in
          pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = combinedFilter;
          };

        # Native dependencies required by C-linking crates
        buildInputs =
          [
            pkgs.openssl
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.cmake # needed by some -sys crates
        ];

        commonArgs = {
          inherit src buildInputs nativeBuildInputs;
          strictDeps = true;
          pname = "provenant";
          version = "0.0.13";
        };

        # Build only the cargo dependencies for caching / CI reuse
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs
          // {
            pname = "provenant-deps";
          });

        # The actual binary crate
        provenant = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            # Tests reference large testdata/ fixture directories via include_str!
            # that are not part of the filtered source. Run tests via `cargo test`
            # in the devShell instead.
            doCheck = false;
          });

        # Clippy check as a separate derivation
        provenantClippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        # Format check
        provenantFmt = craneLib.cargoFmt {
          inherit src;
        };
      in {
        packages = {
          default = provenant;
          inherit provenant;
        };

        checks = {
          inherit provenant provenantClippy provenantFmt;
        };

        devShells.default = craneLib.devShell {
          # Inherit build inputs from the package
          inputsFrom = [provenant];

          # Extra tools for interactive development
          packages = with pkgs; [
            # Rust tools (cargo, rustc, clippy, rustfmt come from the toolchain)
            rust-analyzer

            # Node.js tooling (for docs linting / formatting)
            nodejs

            # Nix tooling
            alejandra
          ];
        };
      }
    );
}
