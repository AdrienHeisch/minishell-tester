{
  description = "Maxitest for minishell";

  inputs = { nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05"; };

  outputs = { nixpkgs, ... }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in {
      PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
      packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
        name = "maxitest";
        src = ./.;
        cargoLock = { lockFile = ./Cargo.lock; };
        buildInputs = with pkgs; [ openssl ];
        nativeBuildInputs = with pkgs; [ pkg-config ];
      };
    };
}
