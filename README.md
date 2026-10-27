# g75-cli

Set the lighting colour of a wired MCHOSE G75 Pro from the command line, without the MCHOSE web driver.

```bash
g75-cli color          # print the colour of the current light mode
g75-cli color eb1c34   # set it
```

The colour is saved to the keyboard. Wireless (2.4G) mode is not supported.

## Install

NixOS flake:

```nix
{
  inputs.g75-cli.url = "github:nolight132/g75-cli";

  outputs =
    { nixpkgs, g75-cli, ... }:
    {
      nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
        modules = [
          g75-cli.nixosModules.default
          { hardware.mchose-g75.enable = true; }
        ];
      };
    };
}
```

Elsewhere, `cargo build --release` and copy `70-g75-cli.rules` to `/etc/udev/rules.d/` so it runs without `sudo`.

## License

[MIT](LICENSE)
