import { Buffer } from 'buffer'
import { BN, Program, web3 } from '@project-serum/anchor'

export const Magic = 0xa1b2c3d4
export const Version1 = 1
export const Version = Version1
export const PriceStatus = ['Unknown', 'Trading', 'Halted', 'Auction']
export const CorpAction = ['NoCorpAct']
export const PriceType = ['Unknown', 'Price', 'TWAP', 'Volatility']

const empty32Buffer = Buffer.alloc(32)
const PKorNull = (data: Buffer) => (data.equals(empty32Buffer) ? null : new web3.PublicKey(data))

interface ICreatePriceFeed {
  oracleProgram: Program
  initPrice: number
  confidence?: BN
  expo?: number
}
export const createPriceFeed = async ({
  oracleProgram,
  initPrice,
  confidence,
  expo = -4,
}: ICreatePriceFeed) => {
  const conf = confidence || new BN((initPrice / 10) * 10 ** -expo)
  const collateralTokenFeed = new web3.Account()
  await oracleProgram.rpc.initialize(new BN(initPrice * 10 ** -expo), expo, conf, {
    accounts: { price: collateralTokenFeed.publicKey },
    signers: [collateralTokenFeed],
    instructions: [
      web3.SystemProgram.createAccount({
        fromPubkey: oracleProgram.provider.wallet.publicKey,
        newAccountPubkey: collateralTokenFeed.publicKey,
        space: 1712,
        lamports: await oracleProgram.provider.connection.getMinimumBalanceForRentExemption(1712),
        programId: oracleProgram.programId,
      }),
    ],
  })
  return collateralTokenFeed.publicKey
}
export const setFeedPrice = async (
  oracleProgram: Program,
  newPrice: number,
  priceFeed: web3.PublicKey
) => {
  const info = await oracleProgram.provider.connection.getAccountInfo(priceFeed)
  const data = parsePriceData(info.data)
  await oracleProgram.rpc.setPrice(new BN(newPrice * 10 ** -data.exponent), {
    accounts: { price: priceFeed },
  })
}
export const getFeedData = async (oracleProgram: Program, priceFeed: web3.PublicKey) => {
  const info = await oracleProgram.provider.connection.getAccountInfo(priceFeed)
  return parsePriceData(info.data)
}

export const parseMappingData = (data: Buffer) => {
  // Pyth magic number.
  const magic = data.readUInt32LE(0)
  // Program version.
  const version = data.readUInt32LE(4)
  // Account type.
  const type = data.readUInt32LE(8)
  // Account used size.
  const size = data.readUInt32LE(12)
  // Number of product accounts.
  const numProducts = data.readUInt32LE(16)
  // Unused.
  // const unused = accountInfo.data.readUInt32LE(20)
  // TODO: check and use this.
  // Next mapping account (if any).
  const nextMappingAccount = PKorNull(data.slice(24, 56))
  // Read each symbol account.
  let offset = 56
  const productAccountKeys = []
  for (let i = 0; i < numProducts; i++) {
    const productAccountBytes = data.slice(offset, offset + 32)
    const productAccountKey = new web3.PublicKey(productAccountBytes)
    offset += 32
    productAccountKeys.push(productAccountKey)
  }
  return {
    magic,
    version,
    type,
    size,
    nextMappingAccount,
    productAccountKeys,
  }
}

interface ProductAttributes {
  [index: string]: string
}

export const parseProductData = (data: Buffer) => {
  // Pyth magic number.
  const magic = data.readUInt32LE(0)
  // Program version.
  const version = data.readUInt32LE(4)
  // Account type.
  const type = data.readUInt32LE(8)
  // Price account size.
  const size = data.readUInt32LE(12)
  // First price account in list.
  const priceAccountBytes = data.slice(16, 48)
  const priceAccountKey = new web3.PublicKey(priceAccountBytes)
  const product: ProductAttributes = {}
  let idx = 48
  while (idx < data.length) {
    const keyLength = data[idx]
    idx++
    if (keyLength) {
      const key = data.slice(idx, idx + keyLength).toString()
      idx += keyLength
      const valueLength = data[idx]
      idx++
      const value = data.slice(idx, idx + valueLength).toString()
      idx += valueLength
      product[key] = value
    }
  }
  return { magic, version, type, size, priceAccountKey, product }
}

const parsePriceInfo = (data: Buffer, exponent: number) => {
  // Aggregate price.
  const priceComponent = data.readBigUInt64LE(0)
  const price = Number(priceComponent) * 10 ** exponent
  // Aggregate confidence.
  const confidenceComponent = data.readBigUInt64LE(8)
  const confidence = Number(confidenceComponent) * 10 ** exponent
  // Aggregate status.
  const status = data.readUInt32LE(16)
  // Aggregate corporate action.
  const corporateAction = data.readUInt32LE(20)
  // Aggregate publish slot.
  const publishSlot = data.readBigUInt64LE(24)
  return {
    priceComponent,
    price,
    confidenceComponent,
    confidence,
    status,
    corporateAction,
    publishSlot,
  }
}

export const parsePriceData = (data: Buffer) => {
  // Pyth magic number.
  const magic = data.readUInt32LE(0)
  // Program version.
  const version = data.readUInt32LE(4)
  // Account type.
  const type = data.readUInt32LE(8)
  // Price account size.
  const size = data.readUInt32LE(12)
  // Price or calculation type.
  const priceType = data.readUInt32LE(16)
  // Price exponent.
  const exponent = data.readInt32LE(20)
  // Number of component prices.
  const numComponentPrices = data.readUInt32LE(24)
  // Unused.
  // const unused = accountInfo.data.readUInt32LE(28)
  // Currently accumulating price slot.
  const currentSlot = data.readBigUInt64LE(32)
  // Valid on-chain slot of aggregate price.
  const validSlot = data.readBigUInt64LE(40)
  // Product id / reference account.
  const productAccountKey = new web3.PublicKey(data.slice(48, 80))
  // Next price account in list.
  const nextPriceAccountKey = new web3.PublicKey(data.slice(80, 112))
  // Aggregate price updater.
  const aggregatePriceUpdaterAccountKey = new web3.PublicKey(data.slice(112, 144))
  const aggregatePriceInfo = parsePriceInfo(data.slice(144, 176), exponent)
  // Urice components - up to 16.
  const priceComponents = []
  let offset = 176
  let shouldContinue = true
  while (offset < data.length && shouldContinue) {
    const publisher = PKorNull(data.slice(offset, offset + 32))
    offset += 32
    if (publisher) {
      const aggregate = parsePriceInfo(data.slice(offset, offset + 32), exponent)
      offset += 32
      const latest = parsePriceInfo(data.slice(offset, offset + 32), exponent)
      offset += 32
      priceComponents.push({ publisher, aggregate, latest })
    } else {
      shouldContinue = false
    }
  }
  return {
    magic,
    version,
    type,
    size,
    priceType,
    exponent,
    numComponentPrices,
    currentSlot,
    validSlot,
    productAccountKey,
    nextPriceAccountKey,
    aggregatePriceUpdaterAccountKey,
    ...aggregatePriceInfo,
    priceComponents,
  }
}
