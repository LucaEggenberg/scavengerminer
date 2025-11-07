# Quickstart

## Running the miner directly from the repo (recommended only for debugging)

### Enter dev shell
```sh
nix develop
```

or if direnv is installed:
```sh
direnv allow
```

### Build
```sh
cargo build --release
```

### Run miner
```sh
RUST_LOG=info \
./target/release/scavenger-miner \
    --network mainnet \
    --keystore /path/to/keystore \ #back this up regularly!
    --enable-donate \
    --donate-to "<your-donate-address>" \
    mine
```

or simply edit `run.sh` specifically change "donate-to" address and enter a valid keystore location for you system.

## Installing the miner as a service (recommended)

### build the flake
```sh
nix build  github:LucaEggenberg/scavengerminer#miner
```

the miner will appear here: `./result/bin/scavenger-miner`

copy it somewhere convenient.

### Running as Service

#### **macOS (Launchd Service)**
Create service file
```bash
~/Library/LaunchAgents/io.midnight.scavenger.plist
```

Paste:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>

    <key>Label</key>
    <string>io.midnight.scavenger</string>

    <key>ProgramArguments</key>
    <array>
        <string>/Users/<USER>/midnight-miner/scavenger-miner</string>
        <string>--network</string>
        <string>mainnet</string>
        <string>--keystore</string>
        <string>keystore</string>
        <string>--enable-donate</string>
        <string>--donate-to</string>
        <string><DONATION_ADDRESS></string>
        <string>mine</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>NETWORK</key><string>mainnet</string>
        <key>RUST_LOG</key><string>info</string>
        <key>KEYSTORE</key><string>keystore</string>
        <key>SCAVENGER_API</key><string>https://scavenger.prod.gd.midnighttge.io</string>
    </dict>

    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>

    <key>StandardOutPath</key>
    <string>/Users/<USER>/Library/Logs/scavenger.out</string>

    <key>StandardErrorPath</key>
    <string>/Users/<USER>/Library/Logs/scavenger.err</string>

</dict>
</plist>
```

make sure you replace `'<USER>'` and `'<DONATION_ADDRESS>'`

Start the service:
```sh
launchctl load ~/Library/LaunchAgents/io.midnight.scavenger.plist
launchctl start io.midnight.scavenger
```

To stop the service:
```sh
launchctl stop io.midnight.scavenger
launchctl unload ~/Library/LaunchAgents/io.midnight.scavenger.plist
```

To view logs:
```sh
tail -f ~/Library/Logs/scavenger.out
```
---

#### **Linux (not NixOS)**
```sh
sudo install -m755 ./result/bin/scavenger-miner /usr/local/bin/
```

example systemd file:
`/etc/systemd/system/scavenger-miner.service`
```ini
[Unit]
Description=Midnight scavenger miner
After=network-online.target

[Service]
Environment=NETWORK=mainnet
Environment=RUST_LOG=info
Environment=KEYSTORE=keystore_mainnet
Environment=SCAVENGER_API=https://scavenger.prod.gd.midnighttge.io
ExecStart=/usr/local/bin/scavenger-miner --network mainnet --keystore keystore_mainnet --enable-donate --donate-to DONATION_ADDRESS mine
Restart=always
RestartSec=5
Nice=10

[Install]
WantedBy=multi-user.target
```

---
#### **NixOS**
Add the repo to your flake inputs:
```nix
miner-src.url = "github:LucaEggenberg/scavengerminer";
```

create systemd service.
```nix
# example service.nix:
{ config, lib, pkgs, inputs, ... }: {
    environment.systemPackages = [
        inputs.miner-src.packages.${pkgs.system}.miner
    ];

    systemd.services.scavenger-miner = {
        description = "Midnight scavenger miner";
        after = [ "network-online.target" ];

        serviceConfig = {
            Environment = [
                "NETWORK=mainnet"
                "RUST_LOG=info"
                "KEYSTORE=/path/to/keystore" # change this!!!
                "SCAVENGER_API=https://scavenger.prod.gd.midnighttge.io"
            ];

            ExecStart = ''
                ${inputs.miner-src.packages.${pkgs.system}.miner}/bin/scavenger-miner \
                    --network mainnet \
                    --keystore /path/to/keystore \ # change this!!!
                    --enable-donate \
                    --donate-to "<your-address>" \ # change this!!!
                    mine
            '';

            Restart = "always";
            RestartSec = "5s";
            Nice = 10;
        };

        wantedBy = [ "multi-user.target" ];
    };
}
```

---

## Want to check your token estimate?
On linux run:
```sh
journalctl -u scavenger-miner --grep='::accounting' -n 1 | sed 's/.*accounting: Accounting — //'
```

on macOS:
```sh
grep '::accounting' ~/Library/Logs/scavenger.out | tail -n 1 | sed 's/.*accounting: Accounting — //'
```

*Note*: token shares are only announced at 00:00 every day. if this is your first day running the miner, the estimate will show 0 NIGHT

## What you can tweak
- `--workers` to scale threads per challenge
- `--keystore ./keystore` location for saved keys
- `--enable-donate` donate mined token to one address if enabled make sure to also configure `--donate-to`
- `--donate-to "<your-donate-address>"` the address the tokens will be donated to.

