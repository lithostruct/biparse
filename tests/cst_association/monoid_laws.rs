
use biparse::{cst::AssociatedGreenCst, green::GreenInterner};
use proptest::proptest;

use super::{SExp, Lang, arbitrary_sexp};


#[test]
fn zero_identity() {
    let left: AssociatedGreenCst<usize, usize, ()> = AssociatedGreenCst::new();
    let right = AssociatedGreenCst::new();
    let merged = left.merge_with(right);
    assert_eq!(merged, AssociatedGreenCst::new());
}

proptest!{
    #[test]
    fn left_identity(sexp in arbitrary_sexp()) {
        let mut supply = Some(0_usize);
        let left = AssociatedGreenCst::new();
        let right = sexp.to_cst(&mut GreenInterner::new(), &mut supply);
        let merged = left.merge_with(right.clone());
        assert_eq!(merged, right);
    }

    #[test]
    fn right_identity(sexp in arbitrary_sexp()) {
        let mut supply = Some(0_usize);
        let left = sexp.to_cst(&mut GreenInterner::new(), &mut supply);
        let right = AssociatedGreenCst::new();
        let merged = left.clone().merge_with(right);
        assert_eq!(merged, left);
    }

    #[test]
    fn associativity(s1 in arbitrary_sexp(), s2 in arbitrary_sexp(), s3 in arbitrary_sexp()) {
        let mut interner = GreenInterner::new();
        let mut supply = Some(0_usize);
        let a = s1.to_cst(&mut interner, &mut supply);
        let b = s2.to_cst(&mut interner, &mut supply);
        let c = s3.to_cst(&mut interner, &mut supply);
        
        let left_nested = (a.clone().merge_with(b.clone())).merge_with(c.clone());
        let right_nested = a.merge_with(b.merge_with(c));

        assert_eq!(left_nested, right_nested)
    }
}