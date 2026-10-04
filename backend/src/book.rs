//! OKX books snapshot/delta sequence checking. No retired checksum dependency.
use anyhow::{Result, ensure};
use rust_decimal::Decimal as D;
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Default, Clone)]
pub struct Book {
    seq: Option<i64>,
    bids: BTreeMap<D, Value>,
    asks: BTreeMap<D, Value>,
}
impl Book {
    pub fn apply(&mut self, action: &str, v: &Value, received: i64) -> Result<Value> {
        let seq = v["seqId"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("seqId ontbreekt"))?;
        let prev = v["prevSeqId"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("prevSeqId ontbreekt"))?;
        ensure!(seq >= 0, "Negatieve sequence");
        let mut next = self.clone();
        if action == "snapshot" {
            ensure!(prev == -1, "Snapshot zonder prevSeqId -1");
            next.bids.clear();
            next.asks.clear();
        } else {
            ensure!(
                action == "update" && self.seq == Some(prev),
                "Orderboek sequence-gap; resync vereist"
            );
            if seq == prev {
                ensure!(
                    v["bids"].as_array().is_some_and(|a| a.is_empty())
                        && v["asks"].as_array().is_some_and(|a| a.is_empty()),
                    "Heartbeat met gewijzigde levels"
                );
            }
        }
        for (side, target) in [("bids", &mut next.bids), ("asks", &mut next.asks)] {
            let levels = v[side]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Boeklevels ontbreken"))?;
            ensure!(levels.len() <= 400, "Boekbufferlimiet");
            for row in levels {
                let price = row[0].as_str().unwrap_or("").parse::<D>()?;
                let qty = row[1].as_str().unwrap_or("").parse::<D>()?;
                ensure!(price > D::ZERO && qty >= D::ZERO, "Ongeldig boekniveau");
                if qty == D::ZERO {
                    target.remove(&price);
                } else {
                    target.insert(price, row.clone());
                }
            }
            ensure!(
                target.len() <= 800,
                "Boekbuffer loopt vol; snapshot vereist"
            );
        }
        if let (Some((bid, _)), Some((ask, _))) =
            (next.bids.last_key_value(), next.asks.first_key_value())
        {
            ensure!(bid < ask, "Gekruist orderboek");
        }
        // OKX may reset sequence downwards; continuity is prev==last, not increasing seq.
        next.seq = Some(seq);
        let out = json!({"bids":next.bids.iter().rev().take(5).map(|(_,v)|v).collect::<Vec<_>>(),"asks":next.asks.values().take(5).collect::<Vec<_>>(),"ts":v["ts"],"seqId":seq,"prevSeqId":prev,"received_at_ms":received,"quality":"CONTIGUOUS","channel":"books"});
        *self = next;
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn msg(seq: i64, prev: i64) -> Value {
        json!({"seqId":seq,"prevSeqId":prev,"ts":"1000","bids":[["99","1","0","1"]],"asks":[["101","2","0","1"]]})
    }
    #[test]
    fn delta_requires_snapshot() {
        assert!(Book::default().apply("update", &msg(2, 1), 1).is_err());
    }
    #[test]
    fn gap_is_rejected_without_changing_state() {
        let mut b = Book::default();
        b.apply("snapshot", &msg(3, -1), 1).unwrap();
        assert!(b.apply("update", &msg(5, 4), 2).is_err());
        assert_eq!(b.seq, Some(3));
    }
    #[test]
    fn reset_with_continuity_accepted() {
        let mut b = Book::default();
        b.apply("snapshot", &msg(8, -1), 1).unwrap();
        assert!(b.apply("update", &msg(1, 8), 2).is_ok());
    }
    #[test]
    fn empty_heartbeat_accepted() {
        let mut b = Book::default();
        b.apply("snapshot", &msg(1, -1), 1).unwrap();
        let h = json!({"seqId":1,"prevSeqId":1,"ts":"2000","bids":[],"asks":[]});
        assert!(b.apply("update", &h, 2).is_ok());
        assert!(b.apply("update", &msg(1, 1), 2).is_err());
    }
    #[test]
    fn deletion_applied() {
        let mut b = Book::default();
        b.apply("snapshot", &msg(1, -1), 1).unwrap();
        let mut v = msg(2, 1);
        v["bids"][0][1] = json!("0");
        let out = b.apply("update", &v, 2).unwrap();
        assert!(out["bids"].as_array().unwrap().is_empty());
    }
}
