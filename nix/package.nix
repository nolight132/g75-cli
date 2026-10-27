{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "g75-cli";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../70-g75-cli.rules
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  postInstall = ''
    install -Dm644 70-g75-cli.rules $out/lib/udev/rules.d/70-g75-cli.rules
  '';

  meta = {
    description = "CLI to set the lighting colour of the MCHOSE G75 Pro";
    homepage = "https://github.com/nolight132/g75-cli";
    license = lib.licenses.mit;
    mainProgram = "g75-cli";
    platforms = lib.platforms.linux;
  };
}
