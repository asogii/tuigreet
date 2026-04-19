{
  self,
  craneLib,
  stdenv,
  taplo,
  cargo-nextest,
}:
craneLib.devShell {
  # Automatically inherit any build inputs from `my-crate`
  inputsFrom = [self.packages.${stdenv.hostPlatform.system}.tuigreet];

  # Also inherit inputs from checks.
  checks = self.checks.${stdenv.hostPlatform.system};

  # Other packages not provided by rust-overlay
  packages = [
    taplo
    cargo-nextest
  ];
}
