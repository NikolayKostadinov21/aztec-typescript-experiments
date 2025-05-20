with import <nixpkgs> {};
mkShell {
  nativeBuildInputs = [
    pkg-config
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${openssl.dev}/lib/pkgconfig";
  '';

}
