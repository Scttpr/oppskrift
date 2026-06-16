{
  description = "Oppskrift — federated recipe platform (Rust/Axum) dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # The `default` profile already bundles cargo, clippy, rustfmt and rust-std.
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          # Host build tools (compilers / code generators) for the *-sys crates.
          nativeBuildInputs = with pkgs; [
            pkg-config # locates system openssl/sqlite/zlib for *-sys crates
            cmake # aws-lc-sys (AWS SDK) builds AWS-LC via CMake
            perl # openssl-sys / aws-lc-sys code generation
          ];

          # Libraries linked into the binary.
          buildInputs =
            with pkgs;
            [
              openssl # reqwest default-tls + lettre native-tls + native-tls crate
              sqlite # libsqlite3-sys (transitive)
              zlib # libz-sys (transitive)
            ]
            ++ lib.optionals stdenv.isDarwin [
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.SystemConfiguration
              darwin.apple_sdk.frameworks.CoreFoundation
            ];

          # Developer CLIs.
          packages = with pkgs; [
            rustToolchain
            sqlx-cli # `sqlx migrate run`, database create/drop
            cargo-audit # mirrors the CI security gate (cargo audit)
            cargo-watch # `make dev` -> cargo watch -x run
            tailwindcss_3 # `make css` — project pins Tailwind v3 (matches Dockerfile)
            postgresql # psql / pg_isready client tools
            gnumake # the project's Makefile
            podman-compose # the Makefile auto-detects this; drives your system rootless podman
            gh # GitHub CLI (PRs, issues, releases)
          ];

          # Make *-sys crates link the Nix OpenSSL instead of vendoring/compiling it.
          OPENSSL_NO_VENDOR = "1";
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            # Seed a local .env so DATABASE_URL / JWT_SECRET / S3_BUCKET exist.
            # NOTE: sqlx::query! macros are checked at COMPILE time, so a migrated
            # Postgres must be reachable at DATABASE_URL before `cargo build`.
            if [ ! -f .env ] && [ -f .env.example ]; then
              cp .env.example .env
              echo "[oppskrift] created .env from .env.example"
            fi

            # The Makefile invokes ./tailwindcss; point it at the Nix binary if absent.
            if [ ! -e ./tailwindcss ]; then
              ln -s "$(command -v tailwindcss)" ./tailwindcss
            fi

            echo "[oppskrift] $(rustc --version)"
            echo "[oppskrift] DB + migrate + run:  make migrate && make dev"
            echo "[oppskrift] storage + email:     podman-compose up -d minio minio-init mailpit"
          '';
        };
      }
    );
}
