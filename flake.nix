{
  description = "Quantframe development shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      runtimeLibs = with pkgs; [
        fontconfig
        freetype
        libxkbcommon
        libxcb
        libx11
        libxcursor
        libxi
        libxrandr
        wayland
        vulkan-loader
        openssl
        zstd
        alsa-lib
      ];
      tauriLibs = with pkgs; [
        webkitgtk_4_1
        gtk3
        libsoup_3
        glib
        cairo
        pango
        gdk-pixbuf
        atk
        librsvg
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages =
          with pkgs;
          [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            cmake
            clang
            mold
            nodejs_22
            pnpm
            xvfb-run
            mesa
            imagemagick
            xdotool
          ]
          ++ runtimeLibs
          ++ tauriLibs;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
        VK_ICD_FILENAMES = "${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64.json";
      };
    };
}
