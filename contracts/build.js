const shell = require('shelljs')

shell.fatal = true; // same as "set -e"

// Names of contracts MUST be same as folder names
const contracts = [
  'auction_house',
  'escrow',
  'account_manager',
]

const cmds = {
  build: 'cargo build --target wasm32-unknown-unknown --release',
  copy: './target/wasm32-unknown-unknown/release/'
}

shell.cd('contracts')

;(async () => {
  let i = 0
  for (const contract of contracts) {
    // Note: see flags in ./cargo/config
    await shell.cd(`${i > 0 ? '../' : ''}${contract}`)
    await shell.exec(`${cmds.build}`)
    await shell.cp(`${contract}.wasm`, '../dist')
    i++
  }
})()
