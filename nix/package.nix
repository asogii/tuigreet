{
  lib,
  craneLib,
  installShellFiles,
  versionCheckHook,
  scdoc,
}: let
  commonArgs = {
    pname = "tuigreet";
    version = (lib.importTOML ../Cargo.toml).package.version;
    src = let
      s = ../.;
      fs = lib.fileset;
    in
      fs.toSource {
        root = s;
        fileset = fs.unions [
          (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) (s + /src))
          (s + /contrib)
          (s + /build.rs)
          (s + /Cargo.lock)
          (s + /Cargo.toml)
          (s + /i18n.toml)
        ];
      };
    strictDeps = true;
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {name = "tuigreet-deps";});
in
  craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
      useNextest = true;

      nativeInstallCheckInputs = [versionCheckHook];
      doInstallCheck = true;

      nativeBuildInputs = [
        installShellFiles
        scdoc
      ];

      postInstall = ''
        scdoc < ${../contrib}/man/tuigreet-1.scd > tuigreet.1
        installManPage tuigreet.1
      '';

      meta = {
        description = "Graphical console greeter for greetd";
        license = lib.licenses.gpl3Only;
        maintainers = with lib.maintainers; [NotAShelf];
        mainProgram = "tuigreet";
      };
    })
