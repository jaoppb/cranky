{
  description = "Cranky, a modular Wayland bar for Hyprland";

  # flake.lock is the pin for every tool below: the only version written here
  # is the tag of jaoppb/tools. `just flake-update` moves the lock, in a pull
  # request of its own.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    tools = {
      url = "github:jaoppb/tools/v0.3.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      tools,
    }:
    let
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );

      # The toolchain rust-toolchain.toml names, so rustup users and the flake
      # move together. cargo-llvm-cov needs the LLVM tools, which only the
      # flake adds: a rustup user who never measures coverage does not
      # download them.
      toolchainFor =
        pkgs:
        (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override (old: {
          extensions = (old.extensions or [ ]) ++ [ "llvm-tools-preview" ];
        });
    in
    {
      packages = forEachSystem (
        pkgs:
        let
          toolchain = toolchainFor pkgs;
          manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        in
        {
          default =
            (pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            }).buildRustPackage
              {
                pname = "cranky";
                inherit (manifest.workspace.package) version;
                src = self;
                cargoLock.lockFile = ./Cargo.lock;
                nativeBuildInputs = [ pkgs.pkg-config ];
                buildInputs = [ pkgs.libxkbcommon ];
                # The gate runs every test inside the devShell; the build
                # sandbox has no session bus and no compositor for them.
                doCheck = false;
                meta = {
                  inherit (manifest.package) description;
                  homepage = manifest.workspace.package.repository;
                  license = pkgs.lib.licenses.mit;
                  mainProgram = "cranky";
                  platforms = pkgs.lib.platforms.linux;
                };
              };
        }
      );

      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [
            (toolchainFor pkgs)
            tools.packages.${pkgs.stdenv.hostPlatform.system}.default
            # `tools` reads and writes GitHub through the gh CLI.
            pkgs.gh
            pkgs.pkg-config
            pkgs.just
            pkgs.lefthook
            pkgs.cargo-deny
            pkgs.cargo-llvm-cov
            pkgs.actionlint
            pkgs.shellcheck
            # genhtml, for `just coverage`.
            pkgs.lcov
            # The integration tests start their own D-Bus daemon and a
            # headless sway, and take screenshots with grim.
            pkgs.dbus
            pkgs.sway
            pkgs.grim
            # `just record-profile`.
            pkgs.samply
            # `just analyze-profile` and scripts/lint-loc.sh.
            pkgs.python3
          ];
          # The only system library the build links against.
          buildInputs = [ pkgs.libxkbcommon ];
          # Checked by the Justfile's `_devshell` guard: recipes run only here.
          CRANKY_DEVSHELL = "1";
        };
      });
    };
}
