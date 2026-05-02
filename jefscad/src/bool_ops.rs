use std::collections::HashMap;

use crate::brep_kernel::{Curve2Id, Curve3Id, FaceId, VertexId};

#[derive(Debug, Clone)]
pub struct FaceFaceIntersection {
    pub v_start:  VertexId,
    pub v_end:    VertexId,
    pub curve3:   Curve3Id,
    pub pcurve_a: Curve2Id,
    pub pcurve_b: Curve2Id,
}

pub struct SsiTable(HashMap<(FaceId, FaceId), Option<FaceFaceIntersection>>);

fn canonical(fa: FaceId, fb: FaceId) -> (FaceId, FaceId) {
    if fa.0 <= fb.0 { (fa, fb) } else { (fb, fa) }
}

impl SsiTable {
    pub fn new() -> Self {
        SsiTable(HashMap::new())
    }

    pub fn insert(&mut self, fa: FaceId, fb: FaceId, result: Option<FaceFaceIntersection>) {
        self.0.insert(canonical(fa, fb), result);
    }

    pub fn get(&self, fa: FaceId, fb: FaceId) -> Option<&Option<FaceFaceIntersection>> {
        self.0.get(&canonical(fa, fb))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ssi_table_canonical_key_symmetry() {
        let fa = FaceId(0);
        let fb = FaceId(1);
        let ffi = FaceFaceIntersection {
            v_start:  VertexId(0),
            v_end:    VertexId(1),
            curve3:   Curve3Id(0),
            pcurve_a: Curve2Id(0),
            pcurve_b: Curve2Id(1),
        };
        let mut table = SsiTable::new();
        table.insert(fa, fb, Some(ffi));
        assert!(matches!(table.get(fb, fa), Some(Some(_))));
    }

    #[test]
    fn ssi_table_three_state_distinguishable() {
        let fa = FaceId(0);
        let fb = FaceId(1);
        let fc = FaceId(2);
        let mut table = SsiTable::new();
        table.insert(fa, fb, None);
        assert!(matches!(table.get(fa, fb), Some(None))); // tested, no intersection
        assert!(matches!(table.get(fa, fc), None));        // not yet computed
    }
}
