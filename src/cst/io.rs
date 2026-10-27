use std::{cmp::Ordering, fmt::Debug, hash::Hash, ops::Range};

use rowan::{GreenNode, Language, NodeOrToken, SyntaxKind, SyntaxNode};

use crate::{
    cst::{
        CstPath, Pathfind,
        association::{AssociationTable, DiagnosticsTable},
    },
    green::{GreenElement, GreenInterner},
    input::Input,
};

/// The mutable output of CST construction, representing a green CST forest and coupled associations.
///
/// Associations contained are guaranteed to be valid for this CST forest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedGreenCst<Id, EId, E>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
{
    /// The forest of green CST tokens/nodes.
    cst: im::Vector<GreenElement>,
    /// The association table between `Id` and paths into `self.cst` to get those nodes.
    associations: AssociationTable<Id>,
    /// The diagnostics table between `Id` and paths into `self.cst` to get those nodes.
    diagnostics: DiagnosticsTable<EId, E>,
}
impl<Id, EId, E> Default for AssociatedGreenCst<Id, EId, E>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Id, EId, E> AssociatedGreenCst<Id, EId, E>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
{
    /// Returns an immutable view into the green CST forest.
    pub const fn cst(&self) -> &im::Vector<GreenElement> {
        &self.cst
    }
    /// Returns an immutable view into the associations table.
    pub const fn associations(&self) -> &im::HashMap<Id, CstPath> {
        self.associations.inner()
    }
    /// Returns an immutable view into the associations table.
    pub const fn diagnostics(&self) -> &im::HashMap<EId, (E, CstPath)> {
        self.diagnostics.inner()
    }
    /// Deconstructs and returns the components of `self`.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        im::Vector<GreenElement>,
        im::HashMap<Id, CstPath>,
        im::HashMap<EId, (E, CstPath)>,
    ) {
        (
            self.cst,
            self.associations.into_inner(),
            self.diagnostics.into_inner(),
        )
    }

    /// Deconstructs and transforms `self` into raw diagnostics, discarding all `EId` embeddings and path information.
    pub fn into_raw_diagnostics(self) -> Vec<E>
    where
        EId: Clone,
        E: Clone,
    {
        self.diagnostics
            .into_inner()
            .into_iter()
            .map(|(_, (e, _))| e)
            .collect()
    }

    /// Constructs an empty CST, with no associations.
    pub fn new() -> Self {
        AssociatedGreenCst {
            cst: im::Vector::new(),
            associations: AssociationTable::new(),
            diagnostics: DiagnosticsTable::new(),
        }
    }

    /// Constructs a CST containing multiple leaf elements from pattern-match information, with no associations.
    pub fn leaves<I>(
        spans: &[(SyntaxKind, Range<usize>)],
        old_cursor: I::Cursor,
        next_cursor: I::Cursor,
        input: &I,
        interner: &mut GreenInterner,
    ) -> Self
    where
        I: Input + ?Sized,
    {
        let mut byte_offset = input.byte_offset(old_cursor);
        let mut leaf_cst: im::Vector<GreenElement> = im::Vector::new();

        // Leading trivia.
        if old_cursor == input.start() {
            match byte_offset.cmp(&0) {
                Ordering::Less => unreachable!(),
                Ordering::Equal => (),
                Ordering::Greater => {
                    // The span (0)..(byte_offset) contains skipped trivia, which should be included.
                    let Some(trivia_kind) = input.skipped_trivia_kind() else {
                        panic!(
                            "Byte span ({0}..{byte_offset}) not covered by contiguous input of type {}!",
                            std::any::type_name::<I>()
                        );
                    };
                    let substring = input.slice_bytes(0, byte_offset);
                    let element: GreenElement = interner.token(trivia_kind, &substring).into();
                    leaf_cst.push_back(element);
                }
            }
        }

        // Meaningful text and interspersed trivia.
        for cst_span in spans {
            let (kind, span) = cst_span;
            match byte_offset.cmp(&span.start) {
                Ordering::Less => {
                    // The span (byte_offset)..(span.start) contains skipped trivia, which should be included.
                    let Some(trivia_kind) = input.skipped_trivia_kind() else {
                        panic!(
                            "Byte span ({byte_offset}..{}) not covered by contiguous input of type {}!",
                            span.start,
                            std::any::type_name::<I>()
                        );
                    };
                    let substring = input.slice_bytes(byte_offset, span.start);
                    let element: GreenElement = interner.token(trivia_kind, &substring).into();
                    leaf_cst.push_back(element);
                }
                // Fallthrough.
                Ordering::Equal => (),
                Ordering::Greater => {
                    // Logically impossible; some `Pattern` implementor messed up.
                    panic!(
                        "Previous end offset {byte_offset} overlaps with next starting offset {}!",
                        span.start
                    );
                }
            }
            // Add the substring of the actual match to the current one.
            let substring = input.slice_bytes(span.start, span.end);
            let element: GreenElement = interner.token(*kind, &substring).into();
            leaf_cst.push_back(element);
            // Mutate `byte_offset`.
            byte_offset = span.end;
        }

        // Trailing trivia up to the next cursor.
        let next_offset = input.byte_offset(next_cursor);
        match byte_offset.cmp(&next_offset) {
            Ordering::Less => {
                // The span (byte_offset)..(next_offset) contains skipped trivia, which should be included.
                let Some(trivia_kind) = input.skipped_trivia_kind() else {
                    panic!(
                        "Byte span ({byte_offset}..{}) not covered by contigious input of type {}!",
                        next_offset,
                        std::any::type_name::<I>()
                    );
                };
                let substring = input.slice_bytes(byte_offset, next_offset);
                let element: GreenElement = interner.token(trivia_kind, &substring).into();
                leaf_cst.push_back(element);
            }
            // Fallthrough, do nothing.
            Ordering::Equal => (),
            Ordering::Greater => {
                // Logically unsound; some `Pattern` implementor messed up.
                panic!(
                    "Last end offset {byte_offset} is greater than the cursor indicating offset {next_offset}!"
                );
            }
        }

        AssociatedGreenCst {
            cst: leaf_cst,
            associations: AssociationTable::new(),
            diagnostics: DiagnosticsTable::new(),
        }
    }
}

impl<Id, EId, E> AssociatedGreenCst<Id, EId, E>
where
    Id: Debug + Clone + Eq + Hash,
    EId: Debug + Clone + Eq + Hash,
    E: Clone,
{
    /// Associatively merges with `self` as the left-hand-side, and a given right-hand-side.
    ///
    /// # Panics
    ///
    /// Panics if the [`IdSupply`](crate::cst::supply::IdSupply) protocol is violated,
    /// such that there exists overlapping entries in both association tables.
    pub fn merge_with(self, rhs: Self) -> Self {
        let merged_associations = self
            .associations
            .merge_with(rhs.associations, self.cst.len());
        let merged_diagnostics = self.diagnostics.merge_with(rhs.diagnostics, self.cst.len());
        let merged_forest = self.cst + rhs.cst;
        AssociatedGreenCst {
            cst: merged_forest,
            associations: merged_associations,
            diagnostics: merged_diagnostics,
        }
    }
    /// Creates a new parent taking the forest of `self` as children.
    pub fn attach_parent(self, kind: SyntaxKind, interner: &mut GreenInterner) -> Self {
        let new_associations = self.associations.with_new_parent(0);
        let new_diagnostics = self.diagnostics.with_new_parent(0);
        let new_node = interner.node(kind, self.cst);
        AssociatedGreenCst {
            cst: im::Vector::unit(new_node.into()),
            associations: new_associations,
            diagnostics: new_diagnostics,
        }
    }

    /// Associates a root node at the given index of the forest with the corresponding Id.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.green.len()` or if `with_id` already exists in the association table.
    pub fn associate_root_at(mut self, index: usize, with_id: Id) -> Self {
        if index >= self.cst.len() {
            panic!(
                "Cannot find node {index}; forest is of length {}!",
                self.cst.len()
            )
        }
        match self
            .associations
            .insert(with_id.clone(), CstPath::here_at(index))
        {
            None => (),
            Some(existing) => panic!(
                "IdSupply invariant invalidated: {with_id:?} has existing mapping to {existing:?}!"
            ),
        }
        self
    }
    /// Associates a root node as the new parent of the forest, with the corresponding Id.
    ///
    /// # Panics
    ///
    /// Panics if `with_id` already exists in the association table.
    pub fn associate_new_parent(
        mut self,
        kind: SyntaxKind,
        with_id: Id,
        interner: &mut GreenInterner,
    ) -> Self {
        self = self.attach_parent(kind, interner);
        self = self.associate_root_at(0, with_id);
        self
    }

    /// Diagnoses a root node at the given index of the forest with the corresponding EId and diagnostic.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.green.len()` or if `with_id` already exists in the diagnostic table.
    pub fn diagnose_root_at(mut self, index: usize, with_id: EId, diagnostic: E) -> Self
    where
        E: Debug,
    {
        if index >= self.cst.len() {
            panic!(
                "Cannot find node {index}; forest is of length {}!",
                self.cst.len()
            )
        }
        match self
            .diagnostics
            .insert(with_id.clone(), (diagnostic, CstPath::here_at(index)))
        {
            None => (),
            Some(existing) => panic!(
                "IdSupply invariant invalidated: {with_id:?} has existing mapping to {existing:?}!"
            ),
        }
        self
    }
    /// Constructs and diagnoses a root node as the new parent of the forest, with the corresponding Id and diagnostic.
    ///
    /// # Panics
    ///
    /// Panics if `with_id` already exists in the diagnostics table.
    pub fn diagnose_new_parent(
        mut self,
        kind: SyntaxKind,
        with_id: EId,
        diagnostic: E,
        interner: &mut GreenInterner,
    ) -> Self
    where
        E: Debug,
    {
        self = self.attach_parent(kind, interner);
        self = self.diagnose_root_at(0, with_id, diagnostic);
        self
    }

    /// Canonizes `self` as a syntax tree, returning the materialized red node and association table as an [`im::HashMap`].
    ///
    /// Returns `None` if `self.cst.len != 1`, or the current root is a `GreenToken`.
    pub fn canonize<L>(&self) -> Option<(SyntaxNode<L>, im::HashMap<Id, CstPath>)>
    where
        L: Language,
    {
        // Preconditions.
        if self.cst.len() != 1 || matches!(self.cst[0], NodeOrToken::Token(_)) {
            return None;
        }

        let root = self.cst[0].as_node().cloned().expect("Preconditions met!");
        let red_tree = SyntaxNode::<L>::new_root(root);
        Some((red_tree, self.associations.inner().clone()))
    }
    /// Canonizes `self` as a syntax tree using the given kind as the root node,
    /// returning the materialized red node and association table as an [`im::HashMap`].
    pub fn canonize_with<T>(&self, root: T) -> (SyntaxNode<T>, im::HashMap<Id, CstPath>)
    where
        T: Language<Kind = T>,
    {
        // It doesn't matter that the GreenInterner here is bogus.
        // We don't need to deduplicate the root node.
        let rooted: Self = self
            .clone()
            .attach_parent(T::kind_to_raw(root), &mut GreenInterner::new());
        rooted.canonize().expect("Just appended root node!")
    }

    /// Searches `self` for the CST node associated with the given `Id`, if any.
    pub fn search(&self, id: &Id) -> Option<GreenNode> {
        let path = self.associations.get(id)?;
        Some(
            self.cst
                .search(path)
                .expect("Association table contains invalid path!"),
        )
    }
}

/// The actualization of `AssociatedGreenCst` as a red syntax tree.
pub struct AssociatedRedCst<Id, EId, E, L>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
    L: Language,
{
    /// The actualized red CST.
    cst: SyntaxNode<L>,
    /// The association table between `Id` and paths into `self.cst` to get those nodes.
    associations: AssociationTable<Id>,
    /// The diagnostics table between `EId` and the diagnostic corresponding into `self.cst`.
    diagnostics: DiagnosticsTable<EId, E>,
}

// Can't derive because that places an unecessary bound on `L`. Yeah.
impl<Id, EId, E, L> Debug for AssociatedRedCst<Id, EId, E, L>
where
    Id: Debug + Eq + Hash,
    EId: Debug + Eq + Hash,
    E: Debug,
    L: Language,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssociatedRedCst")
            .field("cst", &self.cst)
            .field("associations", &self.associations)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
impl<Id, EId, E, L> Clone for AssociatedRedCst<Id, EId, E, L>
where
    Id: Clone + Eq + Hash,
    EId: Clone + Eq + Hash,
    E: Clone,
    L: Language,
{
    fn clone(&self) -> Self {
        Self {
            cst: self.cst.clone(),
            associations: self.associations.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }
}
impl<Id, EId, E, L> PartialEq for AssociatedRedCst<Id, EId, E, L>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
    E: PartialEq,
    L: Language,
{
    fn eq(&self, other: &Self) -> bool {
        self.cst == other.cst
            && self.associations == other.associations
            && self.diagnostics == other.diagnostics
    }
}
impl<Id, EId, E, L> Eq for AssociatedRedCst<Id, EId, E, L>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
    E: Eq,
    L: Language,
{
}
impl<Id, EId, E, L> Hash for AssociatedRedCst<Id, EId, E, L>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
    E: Hash,
    L: Language,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cst.hash(state);
        self.associations.hash(state);
        self.diagnostics.hash(state);
    }
}

impl<Id, EId, E, L> AssociatedRedCst<Id, EId, E, L>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
    L: Language,
{
    /// Returns an immutable view into the red CST tree.
    pub const fn cst(&self) -> &SyntaxNode<L> {
        &self.cst
    }
    /// Returns an immutable view into the associations table.
    pub const fn associations(&self) -> &im::HashMap<Id, CstPath> {
        self.associations.inner()
    }
    /// Returns an immutable view into the diagnostics table.
    pub const fn diagnostics(&self) -> &im::HashMap<EId, (E, CstPath)> {
        self.diagnostics.inner()
    }
    /// Deconstructs and returns the components of `self`.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        SyntaxNode<L>,
        im::HashMap<Id, CstPath>,
        im::HashMap<EId, (E, CstPath)>,
    ) {
        (
            self.cst,
            self.associations.into_inner(),
            self.diagnostics.into_inner(),
        )
    }

    /// Deconstructs and transforms `self` into raw diagnostics, discarding all `EId` embeddings and path information.
    pub fn into_raw_diagnostics(self) -> Vec<E>
    where
        EId: Clone,
        E: Clone,
    {
        self.diagnostics
            .into_inner()
            .into_iter()
            .map(|(_, (e, _))| e)
            .collect()
    }

    /// Searches `self` for the CST node associated with the given `Id`, if any.
    pub fn search(&self, id: &Id) -> Option<SyntaxNode<L>> {
        let path = self.associations.get(id)?;
        Some(
            self.cst
                .search(path)
                .expect("Association table contains invalid path!"),
        )
    }
    /// Searches `self` for the diagnostic and CST node associated with the given `EId`, if any.
    pub fn diagnose(&self, id: &EId) -> Option<(&E, SyntaxNode<L>)> {
        let diagnosis = self.diagnostics.get(id)?;
        Some((
            &diagnosis.0,
            self.cst
                .search(&diagnosis.1)
                .expect("Association table contains invalid path!"),
        ))
    }
}
