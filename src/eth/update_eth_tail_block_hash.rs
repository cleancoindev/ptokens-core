use crate::{
    types::Result,
    traits::DatabaseInterface,
    eth::{
        eth_state::EthState,
        eth_constants::ETH_TAIL_LENGTH,
        eth_types::EthBlockAndReceipts,
        eth_database_utils::{
            get_eth_tail_block_from_db,
            get_eth_latest_block_from_db,
            put_eth_tail_block_hash_in_db,
            get_eth_canon_to_tip_length_from_db,
            maybe_get_nth_ancestor_eth_block_and_receipts,
        },
    },
};

fn does_tail_block_require_updating<D>(
    db: &D,
    calculated_tail_block: &EthBlockAndReceipts,
) -> Result<bool>
    where D: DatabaseInterface
{
    trace!("✔ Checking if ETH tail block needs updating...");
    get_eth_tail_block_from_db(db)
        .map(|db_tail_block|
            db_tail_block.block.number <= calculated_tail_block.block.number - 1
        )
}

pub fn maybe_update_eth_tail_block_hash<D>(
    state: EthState<D>
) -> Result<EthState<D>>
    where D: DatabaseInterface
{
    info!("✔ Maybe updating ETH tail block hash...");
    let canon_to_tip_length = get_eth_canon_to_tip_length_from_db(&state.db)?;
    get_eth_latest_block_from_db(&state.db)
        .map(|latest_eth_block| {
            info!(
                "✔ Searching for tail block {} blocks back from tip...",
                canon_to_tip_length + ETH_TAIL_LENGTH,
            );
            maybe_get_nth_ancestor_eth_block_and_receipts(
                &state.db,
                &latest_eth_block.block.hash,
                &(canon_to_tip_length + ETH_TAIL_LENGTH),
            )
        })
        .and_then(|maybe_ancester_block_and_id|
            match maybe_ancester_block_and_id {
                None => {
                    info!(
                        "✔ No {}th ancestor block in db yet ∴ {}",
                        canon_to_tip_length,
                        "not updating tail block hash!",
                    );
                    Ok(state)
                }
                Some(ancestor_block) => {
                    info!(
                        "✔ {}th ancestor block found...",
                        canon_to_tip_length + ETH_TAIL_LENGTH,
                    );
                    match does_tail_block_require_updating(
                        &state.db,
                        &ancestor_block
                    )? {
                        false => {
                            info!("✔ ETH tail block does not require updating");
                            Ok(state)
                        }
                        true => {
                            info!("✔ Updating ETH tail block...");
                            put_eth_tail_block_hash_in_db(
                                &state.db,
                                &ancestor_block.block.hash
                            )
                                .and_then(|_| Ok(state))
                        }
                    }
                }
            }
        )
}
