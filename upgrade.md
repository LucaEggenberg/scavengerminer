# Upgrade Path from a previous version

The Midnight team has activated the donate_to endpoint, which means all generated addresses can now be consolidated into one wallet address, making the clame phase simpler and cleaner.

As the previous version had some false assumptions you need to update the miner in order to utilize the feature. Please read this guide carefully.

## What changed in this Version
1: I implemented a startup loop to perform the donation for all current addresses (in case you never get this far again because of difficulty increase)

2: The signature on the donate-to was wrong, which is the reason you're now probably seeing an error 400 bad-request on every donation instead of the 403 forbidden before the endpoint was activated

## What you need to do
### 1: Backup your keystore
I didn't do anything to modify the keystore, but just to be safe. copy the entire keystore directory somewhere safe.

The keystore contains prove of ownership for all your mined receipts. This prove is needed to make donations.

---

### 2: Donation Address
Previously I wrote that the donation address must not have mined previously. This changed, the target-address must also be registered.

What this means for you:<br>
1. Open the browser you mined in before using this tool
2. Click "Reset Session"
3. You will see the Destination address, copy it and press "Cancel" in the Dialog.

If you're running the miner directly from the terminal or via `run.sh` paste this address in your param. `--donate_to "<here>"`

If you created a service as I recommended, find the line `<string>--donate-to</string>`
and paste the address bellow `<string>here</string>` so it looks like this:
```xml
<string>--enable-donate</string>
<string>--donate-to</string>
<string>addr1.............</string>
```

---

### 3: Stop the miner
```sh
launchctl stop io.midnight.scavenger
launchctl unload ~/Library/LaunchAgents/io.midnight.scavenger.plist
```

---

### 4: Update the miner
In your terminal, navigate to the miner folder and run:
```sh
nix build github:LucaEggenberg/scavengerminer#miner
```

This will update the miner in `./result/bin/scavenger-miner`

---

### 5: Start the miner
In your terminal run:
```sh
launchctl load ~/Library/LaunchAgents/io.midnight.scavenger.plist
launchctl start io.midnight.scavenger
```