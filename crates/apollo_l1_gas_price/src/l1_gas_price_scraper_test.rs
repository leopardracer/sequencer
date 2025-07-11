use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use apollo_l1_gas_price_types::{GasPriceData, MockL1GasPriceProviderClient};
use papyrus_base_layer::{L1BlockHash, L1BlockHeader, MockBaseLayerContract};
use rstest::rstest;
use starknet_api::block::GasPrice;

use crate::l1_gas_price_scraper::{
    L1GasPriceScraper,
    L1GasPriceScraperConfig,
    L1GasPriceScraperError,
};

const BLOCK_TIME: u64 = 2;
const GAS_PRICE: u128 = 42;
const DATA_PRICE: u128 = 137;

fn u64_to_block_hash(value: u64) -> L1BlockHash {
    let mut result = [0u8; 32];
    let bytes = value.to_be_bytes();
    result[24..].copy_from_slice(&bytes);
    result
}

fn create_l1_block_header(block_number: u64) -> L1BlockHeader {
    L1BlockHeader {
        number: block_number,
        timestamp: (block_number * BLOCK_TIME).into(),
        base_fee_per_gas: u128::from(block_number) * GAS_PRICE,
        blob_fee: u128::from(block_number) * DATA_PRICE,
        hash: u64_to_block_hash(block_number),
        parent_hash: u64_to_block_hash(block_number.saturating_sub(1)),
        // If needed, add ..Default::default() here.
    }
}

// Assumes `data` was generated by `create_l1_block_header`.
fn check_gas_prices(data: &GasPriceData) -> bool {
    data.timestamp.0 == data.block_number * BLOCK_TIME
        && data.price_info.base_fee_per_gas == GasPrice(u128::from(data.block_number) * GAS_PRICE)
        && data.price_info.blob_fee == GasPrice(u128::from(data.block_number) * DATA_PRICE)
}

fn setup_scraper(
    end_block: u64,
    expected_number_of_blocks: usize,
) -> L1GasPriceScraper<MockBaseLayerContract> {
    let mut mock_contract = MockBaseLayerContract::new();
    mock_contract.expect_latest_l1_block_number().returning(move |_| Ok(Some(end_block)));
    mock_contract.expect_get_block_header().returning(move |block_number| {
        if block_number >= end_block {
            Ok(None)
        } else {
            Ok(Some(create_l1_block_header(block_number)))
        }
    });

    let mut mock_provider = MockL1GasPriceProviderClient::new();
    mock_provider
        .expect_add_price_info()
        .withf(check_gas_prices)
        .times(expected_number_of_blocks)
        .returning(|_| Ok(()));

    L1GasPriceScraper::new(
        L1GasPriceScraperConfig::default(),
        Arc::new(mock_provider),
        mock_contract,
    )
}

#[tokio::test]
async fn run_l1_gas_price_scraper_single_block() {
    const START_BLOCK: u64 = 0;
    const END_BLOCK: u64 = 1;
    const EXPECT_NUMBER: usize = 1;
    let mut scraper = setup_scraper(END_BLOCK, EXPECT_NUMBER);
    scraper.update_prices(START_BLOCK).await.unwrap();
}

#[tokio::test]
async fn run_l1_gas_price_scraper_two_blocks() {
    const START_BLOCK: u64 = 2;
    const END_BLOCK1: u64 = 7;
    const END_BLOCK2: u64 = 12;

    // Explicitly making the mocks here, so we can customize them for the test.
    let mut mock_contract = MockBaseLayerContract::new();
    // Note the order of the expectation is important! Can only scrape the first blocks first.
    mock_contract.expect_latest_l1_block_number().returning(move |_| Ok(Some(END_BLOCK2)));
    mock_contract
        .expect_get_block_header()
        .times(usize::try_from(END_BLOCK1 - START_BLOCK + 1).unwrap())
        .returning(move |block_number| {
            if block_number >= END_BLOCK1 {
                Ok(None)
            } else {
                Ok(Some(create_l1_block_header(block_number)))
            }
        });
    mock_contract
        .expect_get_block_header()
        .times(usize::try_from(END_BLOCK2 - END_BLOCK1 + 1).unwrap())
        .returning(move |block_number| {
            if block_number >= END_BLOCK2 {
                Ok(None)
            } else {
                Ok(Some(create_l1_block_header(block_number)))
            }
        });
    let mut mock_provider = MockL1GasPriceProviderClient::new();
    mock_provider
        .expect_add_price_info()
        .withf(check_gas_prices)
        .times(usize::try_from(END_BLOCK2 - START_BLOCK).unwrap())
        .returning(|_| Ok(()));

    let mut scraper = L1GasPriceScraper::new(
        L1GasPriceScraperConfig::default(),
        Arc::new(mock_provider),
        mock_contract,
    );

    let block_number = scraper.update_prices(START_BLOCK).await.unwrap();
    assert_eq!(block_number, END_BLOCK1);
    let block_number = scraper.update_prices(block_number).await.unwrap();
    assert_eq!(block_number, END_BLOCK2);
}

#[tokio::test]
async fn run_l1_gas_price_scraper_multiple_blocks() {
    const START_BLOCK: u64 = 5;
    const END_BLOCK: u64 = 10;
    const EXPECT_NUMBER: usize = 5;
    let mut scraper = setup_scraper(END_BLOCK, EXPECT_NUMBER);

    // Should update prices from 5 to 10 (not inclusive) and on 10 get a None from base layer.
    scraper.update_prices(START_BLOCK).await.unwrap();
}

#[tokio::test]
async fn l1_reorg_gas_price_scraper_error() {
    const START_BLOCK: u64 = 0;
    // After END_BLOCK1, the scraper will reach the last block and stop.
    const END_BLOCK1: u64 = 2;
    // We will manually create a reorg, and the scraper will continue scraping from END_BLOCK1 to
    // END_BLOCK2.
    const END_BLOCK2: u64 = 4;

    // Explicitly making the mocks here, so we can customize them for the test.
    let mut mock_contract = MockBaseLayerContract::new();
    // Note the order of the expectation is important! Can only scrape the first blocks first.
    mock_contract.expect_latest_l1_block_number().returning(move |_| Ok(Some(END_BLOCK2)));
    mock_contract
        .expect_get_block_header()
        .times(usize::try_from(END_BLOCK1 - START_BLOCK + 1).unwrap())
        .returning(move |block_number| {
            // Return None to tell the scraper to stop scraping.
            if block_number >= END_BLOCK1 {
                Ok(None)
            } else {
                Ok(Some(create_l1_block_header(block_number)))
            }
        });
    // After scraping the first set of blocks, this set will contain a reorg.
    mock_contract
        .expect_get_block_header()
        .times(usize::try_from(END_BLOCK2 - END_BLOCK1).unwrap())
        .returning(move |block_number| {
            // Return None to tell the scraper to stop scraping.
            if block_number >= END_BLOCK2 {
                Ok(None)
            } else {
                let mut header = create_l1_block_header(block_number);
                // Simulate a reorg by assigning an unexpected hash.
                // The parent hash of the next block will still be the block number-1,
                // as it is generated by create_l1_block_header.
                header.hash = u64_to_block_hash(block_number + 100);
                Ok(Some(header))
            }
        });
    let mut mock_provider = MockL1GasPriceProviderClient::new();
    mock_provider
        .expect_add_price_info()
        .withf(check_gas_prices)
        .times(usize::try_from(END_BLOCK2 - START_BLOCK - 1).unwrap())
        .returning(|_| Ok(()));

    let mut scraper = L1GasPriceScraper::new(
        L1GasPriceScraperConfig::default(),
        Arc::new(mock_provider),
        mock_contract,
    );
    // The first call should succeed.
    let result = scraper.update_prices(START_BLOCK).await;
    assert!(result.is_ok());
    // The second call should fail with a reorg error.
    let result = scraper.update_prices(END_BLOCK1).await;
    assert!(matches!(result, Err(L1GasPriceScraperError::L1ReorgDetected { .. })));
}

#[rstest]
#[case::high_finality(3)]
#[case::low_finality(1)]
#[tokio::test]
async fn l1_short_reorg_gas_price_scraper_is_fine(#[case] finality: u64) {
    const START_BLOCK: u64 = 0;
    const END_BLOCK: u64 = 10;
    const REORG_BLOCK: u64 = 9;

    let end_of_chain = Arc::new(AtomicU64::new(END_BLOCK));
    let end_of_chain_clone = end_of_chain.clone();
    let has_reorg_happened = Arc::new(AtomicBool::new(false));
    let has_reorg_happened_clone = has_reorg_happened.clone();

    // Returns a spoof hash, based on the block number and whether a reorg happened.
    fn block_hash_calculator(block_number: u64, is_reorg: bool) -> L1BlockHash {
        let mut hash_number = block_number;
        if is_reorg && block_number >= REORG_BLOCK {
            // If a reorg happened, we change the hash number, but only for blocks after
            // REORG_BLOCK.
            hash_number += 100;
        }
        u64_to_block_hash(hash_number)
    }

    // Explicitly making the mocks here, so we can customize them for the test.
    let mut mock_contract = MockBaseLayerContract::new();
    // This expectation just returns the last block number we want (which is end_of_chain-finality).
    mock_contract
        .expect_latest_l1_block_number()
        .returning(move |finality| Ok(Some(end_of_chain_clone.load(Ordering::SeqCst) - finality)));
    // This expectation will return the regular chain, or the chain with the reorg (depending on
    // has_reorg_happened).
    mock_contract.expect_get_block_header().returning(move |block_number| {
        // We never return None, since latest_l1_block_number will stop earlier, due to finality.
        let reorg = has_reorg_happened_clone.load(Ordering::SeqCst);

        let mut header = create_l1_block_header(block_number);
        header.hash = block_hash_calculator(block_number, reorg);
        header.parent_hash = block_hash_calculator(block_number.saturating_sub(1), reorg);
        Ok(Some(header))
    });
    let mut mock_provider = MockL1GasPriceProviderClient::new();
    mock_provider.expect_add_price_info().withf(check_gas_prices).returning(|_| Ok(()));

    // Make a scraper with the finality set.
    let mut scraper = L1GasPriceScraper::new(
        L1GasPriceScraperConfig { finality, ..Default::default() },
        Arc::new(mock_provider),
        mock_contract,
    );
    // The first call should succeed.
    let result = scraper.update_prices(START_BLOCK).await;
    // Successfully scraped the first blocks (we don't reach END_BLOCK, because of finality).
    let new_block_number = result.unwrap();
    assert_eq!(new_block_number, END_BLOCK - finality + 1);

    // Now we simulate a reorg by setting has_reorg_happened to true.
    has_reorg_happened.store(true, Ordering::SeqCst);
    // We allow the chain to keep going to a higher block number.
    end_of_chain.store(END_BLOCK + finality * 2, Ordering::SeqCst);
    // The second call should succeed, as the scraper will handle the reorg.
    let result = scraper.update_prices(new_block_number).await;

    if finality > 1 {
        // High finality case, means we can safely skip over this short reorg.
        let final_block_number = result.unwrap();
        // The final block number should be one after the end of the chain minus finality.
        assert_eq!(final_block_number, end_of_chain.load(Ordering::SeqCst) - finality + 1);
    } else {
        // Low finality case, means we will trigger a reorg error.
        assert!(matches!(result, Err(L1GasPriceScraperError::L1ReorgDetected { .. })));
    }
}

// TODO(guyn): test scraper with a provider timeout
