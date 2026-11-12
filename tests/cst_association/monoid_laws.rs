//! Tests that merging of associated green forests forms a monoid, and that association paths are preserved.

use biparse::{
    cst::{AssociatedGreenCst, Pathfind},
    green::GreenInterner,
};
use proptest::{arbitrary::any, proptest};
use rowan::{GreenNode, SyntaxKind};

use super::sexp_strategy;

#[test]
fn zero_identity() {
    let left: AssociatedGreenCst<usize, usize, ()> = AssociatedGreenCst::new();
    let right = AssociatedGreenCst::new();
    let merged = left.merge_with(right);
    assert_eq!(merged, AssociatedGreenCst::new());
}

proptest! {
    #[test]
    fn left_identity(sexp in sexp_strategy()) {
        let mut supply = Some(0_usize);
        let left = AssociatedGreenCst::new();
        let right = sexp.to_cst(&mut GreenInterner::new(), &mut supply);
        let merged = left.merge_with(right.clone());
        assert_eq!(merged, right);
    }

    #[test]
    fn right_identity(sexp in sexp_strategy()) {
        let mut supply = Some(0_usize);
        let left = sexp.to_cst(&mut GreenInterner::new(), &mut supply);
        let right = AssociatedGreenCst::new();
        let merged = left.clone().merge_with(right);
        assert_eq!(merged, left);
    }

    #[test]
    fn associativity(s1 in sexp_strategy(), s2 in sexp_strategy(), s3 in sexp_strategy()) {
        let mut interner = GreenInterner::new();
        let mut supply = Some(0_usize);
        let a = s1.to_cst(&mut interner, &mut supply);
        let b = s2.to_cst(&mut interner, &mut supply);
        let c = s3.to_cst(&mut interner, &mut supply);

        let left_nested = (a.clone().merge_with(b.clone())).merge_with(c.clone());
        let right_nested = a.merge_with(b.merge_with(c));

        assert_eq!(left_nested, right_nested)
    }

    #[test]
    fn merge_conserves_paths(s1 in sexp_strategy(), s2 in sexp_strategy()) {
        let mut interner = GreenInterner::new();
        let mut supply = Some(0_usize);
        let a = s1.to_cst(&mut interner, &mut supply);
        let b = s2.to_cst(&mut interner, &mut supply);

        let a_mapping : im::OrdMap<usize, GreenNode> = a.associations().clone().into_iter()
            .map(|(id, path)| (id, a.forest().search(&path).unwrap()))
            .collect();
        let b_mapping : im::OrdMap<_, _> = b.associations().clone().into_iter()
            .map(|(id, path)| (id, b.forest().search(&path).unwrap()))
            .collect();

        let merged = a.merge_with(b);
        let merged_mapping : im::OrdMap<_, _> = merged.associations().clone().into_iter()
            .map(|(id, path)| (id, merged.forest().search(&path).unwrap()))
            .collect();

        assert_eq!(a_mapping.union_with_key(b_mapping, |k, v1, v2| panic!("{k} => {v1}, {v2}!")), merged_mapping)
    }

    #[test]
    fn attachment_conserves_paths(sexp in sexp_strategy(), u in any::<u16>()) {
        let mut interner = GreenInterner::new();
        let mut supply = Some(0_usize);
        let cst = sexp.to_cst(&mut interner, &mut supply);

        let mapping : im::OrdMap<usize, GreenNode> = cst.associations().clone().into_iter()
            .map(|(id, path)| (id, cst.forest().search(&path).unwrap()))
            .collect();

        let parented = cst.attach_parent(SyntaxKind(u), &mut interner);
        let parented_mapping : im::OrdMap<_, _> = parented.associations().clone().into_iter()
            .map(|(id, path)| (id, parented.forest().search(&path).unwrap()))
            .collect();

        assert_eq!(mapping, parented_mapping)
    }
}
