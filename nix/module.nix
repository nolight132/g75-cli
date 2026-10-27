{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.hardware.mchose-g75;
in
{
  options.hardware.mchose-g75 = {
    enable = lib.mkEnableOption "lighting control for the MCHOSE G75 Pro";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = lib.literalExpression "pkgs.callPackage ./package.nix { }";
      description = "The g75-cli package to install.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    services.udev.packages = [ cfg.package ];
  };
}
