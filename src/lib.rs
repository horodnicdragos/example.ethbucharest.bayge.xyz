// This contract is used as the reference via The Wizard on Stylus. It
// should be changed to compete in the hackathon!

extern crate alloc;

use stylus_sdk::{alloy_primitives::*, prelude::*};

use alloc::{collections::BTreeMap, vec, vec::Vec};

use libbucharesthashing::{immutables::*, prover, prover::Piece};

/* ~~~~~~~~~~~~~~ BOARD IMPLEMENTATION ~~~~~~~~~~~~~~ */

/// Board that this game is played on. Could be of any size. This could
/// be optimised for gas golfing.
pub type Board = BTreeMap<u32, (Piece, u32)>;

fn in_check_threats(
    search_start: u32,
    search_end: u32,
    board: &Board,
    row_size: u32,
    king_pos: u32,
) -> Vec<u32> {
    let mut threats = vec![];
    for i in search_start..search_end {
        if let Some((piece, piece_pos)) = board.get(&i) {
            if is_solved(row_size, king_pos, *piece_pos, *piece) {
                threats.push(i);
            }
        }
    }
    threats
}

fn is_solved(row_size: u32, king_pos: u32, piece_pos: u32, piece: Piece) -> bool {
    let (piece_x, piece_y, king_x, king_y) = (piece_pos % row_size, piece_pos / row_size, king_pos % row_size, king_pos / row_size);

    // Optimize absolute difference calculation using XOR and wrapping_sub
    let dx = (king_x ^ piece_x).wrapping_sub((king_x < piece_x) as u32);
    let dy = (king_y ^ piece_y).wrapping_sub((king_y < piece_y) as u32);

    // Early return if piece is at same position as king
    if (dx | dy) == 0 {
        return false;
    }

    match piece {
        Piece::PAWN => {
            // piece_y + 1 == king_y && dx == 1
            (king_y == piece_y + 1) & (dx == 1)
        },
        Piece::CASTLE => {
            // dx == 0 || dy == 0
            (dx & dy) == 0 && (dx | dy) > 0
        },
        Piece::QUEEN => {
            // dx == 0 || dy == 0 || dx == dy
            (dx & dy) == 0 || dx == dy
        },
        Piece::BISHOP => dx == dy,
        Piece::KNIGHT => {
            // dx * dy == 2
            // More efficient than multiplication
            ((dx == 2 && dy == 1) || (dx == 1 && dy == 2))
        },
        Piece::KING => {
            // dx <= 1 && dy <= 1
            (dx | dy) <= 1
        },
    }
}

pub fn solve(starting_hash: &[u8], start: u32) -> Option<(u32, u32)> {
    let row_size = BOARD_SIZE.isqrt();
    let mut board = BTreeMap::new();
    let mut last_king = None;
    let mut threats = vec![];
    for i in start..MAX_TRIES {
        let e = prover::hash(starting_hash, i);
        let p_id: u8 = (e % 6).try_into().unwrap();
        let p = Piece::try_from(p_id).unwrap();
        let offset: u32 = (e >> 32).try_into().unwrap();
        let pos: u32 = offset % BOARD_SIZE;
        board.insert(i, (p, pos));
        if p == Piece::KING {
            last_king = Some((pos, i));
            threats = in_check_threats(start, i, &board, row_size, pos);
        } else if let Some((last_king_pos, last_king_nonce)) = last_king {
            {
                let solved = is_solved(row_size, last_king_pos, pos, p);

                if solved {
                    threats.push(i);
                }
            }
        }

        if let Some((_last_king_pos, last_king_nonce)) = last_king {
            if threats.len() >= CHECKS_NEEDED as usize {
                threats.push(last_king_nonce);
                let first_threat = *threats.iter().min().unwrap();
                println!("first_threat: {:?}", threats);
                return Some((first_threat, i));
            }
        }
    }
    None
}

/* ~~~~~~~~~~~~~~ CONTRACT ENTRYPOINT ~~~~~~~~~~~~~~ */

#[storage]
#[entrypoint]
pub struct Storage {}

#[public]
impl Storage {
    // We need to provide this function for the prover contract to check this
    // contract's performance with this function.
    pub fn prove(&self, hash: FixedBytes<32>, from: u32) -> Result<(u32, u32), Vec<u8>> {
        Ok(solve(hash.as_slice(), from).unwrap())
    }
}

/* ~~~~~~~~~~~~~~ ALGORITHM TESTING ~~~~~~~~~~~~~~ */

// This test code will randomly slam the function to test if it behaves
// consistently. It will create hashes for the test function.

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_solve(
            starting_hash in any::<[u8; 64]>()
        ) {
            let (l, h) = solve(&starting_hash, 0).unwrap();
            assert_eq!(
                (l, h),
                solve(&starting_hash, l).unwrap()
            );
        }
    }
}
