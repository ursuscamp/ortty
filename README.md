## About The Project

`ortty` is a Bitcoin block explorer for the terminal with a focus on Ordinals Inscriptions. It has an interactive UI that allows you to explore the Blockchain in real time, view inscriptions (including images) in the terminal, open them on [ordinals.com](https://ordinals.com), etc. It also has a scriptable CLI that allow you to view, extract and filter inscriptions with shell commands.

## Getting Started

1. You must be running a Bitcoin Core node, preferably with `txindex=1` (though not strictly required).
2. `ortty` must be able to connect to your node using either a username/password or the Bitcoin Core cookie file.
   You may specify this information on the command line with `--host <USER>`, `--user <USER>`, `--password <PASSWORD>` and `--cookie <PATH>`.
   If you do not specify a path for the cookie, it will search known folders. They can also be passed in environment variables: `BITCOIN_HOST`,
   `BITCOIN_USER`, `BITCOIN_PASS` and `BITCOIN_COOKIE`.
3. If you have a `.env` file in the current working directory, `ortty` will read the environment variables from that file as well.

## How To Use: Interactive Block Explorer

Enter the interactive block explorer by running `ortty explore`. You will be presented with various menu options, which can be navigated and selected using the `<ENTER>` key:

* `View Blocks` will show you the Bitcoin blocks in descending order from most recent. Selecting a block will present a further menu with every inscription located in that black. Navigate the inscriptions and view them one at a time by hitting `<ENTER>` again.
* `Inscription Filters` give you a list of inscription types which you can filter with. Current options are `Text` for any plain text, `JSON` for any JSON inscriptions, `BRC-20` for any BRC-20-specific inscriptions, `HTML` for known HTML inscriptions, and finally `Image` for any image based inscriptions. All of these options are selected by default. **Note**: In most cases, `ortty` does not trust the inscriptions media type, but instead uses heuristics to guess the images files type.
* `Extra Options` has a few useful additional features. You can tell `ortty` to extract any inscriptions you view interactively to the current working folder, using the format `<INSCRIPTION_ID>.<guessed file extension>`. You can also tell `ortty` to open any inscriptions you view on the web.

## How To Use: CLI

There are two CLI commands: `inscription` and `scan`. To view a single inscription, you can run `ortty inscription <inscription_id>` and it will display the inscription in the terminal and exit. This requires your connected node has `txindex=1` set.

The command `scan` is more complicated and more useful. It can scan a block, transaction or transaction input for all inscriptions and outputs them in various ways. You can use a combination of the `--block <BLOCK HASH OR BLOCK HEIGHT>` and `--tx <TXID>` to scan. If you do not specify `--tx` then it will scan the whole block. You can specify __only__ `--tx` if your node runs with the `txindex=1` option.

Additionally, you can use `--web` to open the transaction on the [Ordinals indexer](https://ordinals.com). You can use `--extract <FOLDER>` to extract the the inscriptions to an output folder. It will use heuristics to guess the appropriate file extension and it take the name `<INSCRIPTION_ID>.<extension>`. You can use `--filter <FILTER>` to filter the inscriptions by different types: `text`, `json`, `brc20` and `image`. You can specify `--filter` multiples times and it will treat them as an `OR` filter.
