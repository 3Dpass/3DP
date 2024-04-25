use sp_core::H256;
use sp_std::vec::Vec;
use sp_std::str::from_utf8;
use sp_std::collections::btree_set;

pub(crate) fn check_obj(_alg_id: &[u8;16], _obj: &Vec<u8>, _hashes: &Vec<H256>) -> bool {

    use sp_std::vec;
    use sp_consensus_poscan::decompress_obj;

	let mut obj = _obj.clone();

	if obj[..4] == vec![b'l', b'z', b's', b's'] {
		obj = decompress_obj(&obj[4..]);
	}

	check_simply_connected(&obj)
}

fn check_simply_connected(obj: &Vec<u8>) -> bool {
    if let Ok(text) = from_utf8(&obj) {
        let mut vs_list = btree_set::BTreeSet::new();
        for line in text.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.len() > 0 {
                match words[0] {
                    "fo" | "f" => {
                        if words.len() == 4 {
                            let mut vs = [0,0,0];
                            for idx in 1..=3 {
                                let swords: Vec<&str> = words[idx].split("/").collect();
                                if swords.len() == 3 {
                                    if let Ok(i) = swords[0].parse() {
                                        vs[idx-1] = i;
                                    } else {
										return false;
									}
                                } else {
									return false;
								}
                            }
                            if vs_list.is_empty()
                            || vs_list.contains(&vs[0])
                            || vs_list.contains(&vs[1])
                            || vs_list.contains(&vs[2]) {
                                vs_list.insert(vs[0]);
                                vs_list.insert(vs[1]);
                                vs_list.insert(vs[2]);
                            } else {
                                return false;
                            }
                        } else {
							return false;
						}
                    },
                    _=>{},
                }
            }
        }
        true
    } else {
        false
    }
}