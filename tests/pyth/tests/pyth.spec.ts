import * as anchor from '@project-serum/anchor'
import { BN, Program, web3 } from '@project-serum/anchor'
import assert from 'assert'
import { createPriceFeed, setFeedPrice, getFeedData } from './oracleUtils'

describe('pyth-oracle', () => {
  anchor.setProvider(anchor.Provider.env())
  const program = anchor.workspace.Pyth as Program

  it('initialize', async () => {
    const price = 50000
    const priceFeedAddress = await createPriceFeed({
      oracleProgram: program,
      initPrice: price,
      expo: -6,
    })
    const feedData = await getFeedData(program, priceFeedAddress)
    assert.ok(feedData.price === price)
  })

  it('change feed price', async () => {
    const price = 50000
    const expo = -7
    const priceFeedAddress = await createPriceFeed({
      oracleProgram: program,
      initPrice: price,
      expo: expo,
    })
    const feedDataBefore = await getFeedData(program, priceFeedAddress)
    assert.ok(feedDataBefore.price === price)
    assert.ok(feedDataBefore.exponent === expo)

    const newPrice = 55000
    await setFeedPrice(program, newPrice, priceFeedAddress)
    const feedDataAfter = await getFeedData(program, priceFeedAddress)
    assert.ok(feedDataAfter.price === newPrice)
    assert.ok(feedDataAfter.exponent === expo)
  })
})
