//! The faceting algorithm.

use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque}, vec, iter::FromIterator, io::Write, time::Instant, path::PathBuf};

use crate::{
    abs::{Abstract, Element, ElementList, Ranked, Ranks, Subelements, Superelements, AbstractBuilder},
    conc::{Concrete, ConcretePolytope},
    float::Float,
    group::{Group}, geometry::{Matrix, PointOrd, Subspace, Point}, Polytope
};

use ordered_float::OrderedFloat;

use vec_like::*;

/// Input for the faceting function
pub enum GroupEnum {
    /// Group of matrices
    ConcGroup(Group<vec::IntoIter<Matrix<f64>>>),
    /// Group of vertex mappings
    VertexMap(Vec<Vec<usize>>),
    /// True: take chiral group
    /// False: take full group
    Chiral(bool),
}

const CL: &str = "\r                                                                                                                   \r";

const DELAY: u128 = 200;

impl Ranks {
    /// Sorts some stuff in a way that's useful for the faceting algorithm.
    pub fn element_sort_strong(&mut self) {
        for el in 0..self[2].len() {
            self[2][el].subs.sort_unstable();
        }

        for rank in 2..self.len()-1 {
            let mut all_subs = Vec::new();
            for el in &self[rank] {
                all_subs.push(el.subs.clone());
            }
            let mut sorted = all_subs.clone();
            sorted.sort_unstable();

            let mut perm = Vec::new();
            for i in &all_subs {
                perm.push(sorted.iter().position(|x| x == i).unwrap());
            }

            for i in 0..self[rank].len() {
                self[rank][i].subs = sorted[i].clone();
                self[rank][i].subs.sort_unstable();
            }

            let mut new_list = ElementList::new();
            for i in 0..self[rank+1].len() {
                let mut new = Element::new(Subelements::new(), Superelements::new());
                for sub in &self[rank+1][i].subs {
                    new.subs.push(perm[*sub]);
                }
                new.sort();
                new_list.push(new);
            }
            self[rank+1] = new_list;
        }
    }

    /// Sorts some stuff in a way that's useful for the faceting algorithm.
    pub fn element_sort_strong_with_local(&mut self, local: &Ranks) {
        for el in 0..self[2].len() {
            self[2][el].subs.sort_unstable();
        }

        for rank in 2..self.len()-1 {
            let mut all_subs = Vec::new();
            for el in &self[rank] {
                all_subs.push(el.subs.clone());
            }
            let mut sorted = all_subs.clone();
            sorted.sort_unstable();

            let mut perm = Vec::new();
            for i in &all_subs {
                perm.push(sorted.iter().position(|x| x == i).unwrap());
            }

            for i in 0..self[rank].len() {
                self[rank][i].subs = sorted[i].clone();
                self[rank][i].subs.sort_unstable();
            }

            let mut map_to_local = HashMap::new();

            for i in 0..self[rank+1].len() {
                for j in 0..self[rank+1][i].subs.len() {
                    map_to_local.insert(self[rank+1][i].subs[j], local[rank+1][i].subs[j]);
                }
            }

            let mut new_list = ElementList::new();
            for i in 0..self[rank+1].len() {
                let mut new = Element::new(Subelements::new(), Superelements::new());
                for sub in &self[rank+1][i].subs {
                    new.subs.push(perm[*map_to_local.get(sub).unwrap()]);
                }
                new.sort();
                new_list.push(new);
            }
            self[rank+1] = new_list;
        }
    }

    /*
    /// Combines two `Ranks`. Only meant to be used in the faceting algorithm.
    fn append(&mut self, other: &Ranks) {
        let counts: Vec<usize> = self.iter().map(|x| x.len()).collect();

        for r in 1..=2 {
            for el in &other[r] {
                self[r].push(el.clone());
            }
        }

        for r in 3..self.rank() {
            for el in &other[r] {
                let mut new_el = el.clone();
                for sub in &mut new_el.subs {
                    *sub += counts[r-1];
                }
                for sup in &mut new_el.sups {
                    *sup += counts[r+1];
                }
                self[r].push(new_el.clone());
            }
        }
    }
    */
}

/// Modified binary search that finds the first element whose first element is greater than `min`.
fn binary(vec: &Vec<(usize,usize)>, min: usize) -> usize{
    let mut lo  = -1;
    let mut hi  = vec.len() as isize;
    let mut c = (lo+hi)/2;

    while hi - lo > 1 {
        if vec[c as usize].0 > min {
            hi = c;
        } else {
            lo = c;
        }
        c = (lo+hi)/2;
    }

    hi as usize
}

/// For each faceting, checks if it is a compound of other facetings, and labels it if so.
fn label_irc(vec: &Vec<Vec<(usize,usize)>>) -> HashMap<usize, (usize,usize)> {
    let mut out = HashMap::<usize, (usize,usize)>::new(); // Map of the index of the compound to the indices of the components.

    'a: for a in 0..vec.len() { // `a` is the index of the base set
        for b in 0..vec.len() { // `b` is the index of a potential subset of `a`
            if vec[b].len() >= vec[a].len() { // A strict subset must be smaller than the base.
                continue
            }
            if vec[b][0] > vec[a][0] { // One of the subsets must contain the first facet.
                break
            }
            let mut i = 0;
            for f in &vec[a] { // Searches through `b` to see if all elements are in `a`.
                if &vec[b][i] > f {
                    continue
                }
                if &vec[b][i] < f {
                    break
                }
                i += 1;

                if i >= vec[b].len() { // We've found a subset.
                    let mut complement = Vec::new();

                    let mut j = 0;
                    for g in &vec[a] {
                        if j >= vec[b].len() {
                            complement.push(*g);
                            continue
                        }
                        if &vec[b][j] > g {
                            complement.push(*g);
                            continue
                        }
                        j += 1;
                    }

                    for c in b+1..vec.len() { // Look for its complement.
                        if vec[c] == complement {
                            out.insert(a,(b,c));
                            break
                        }
                    }
                    continue 'a;
                }
            }
        }
    }
    out
}

/// For each faceting, checks if it is a compound of other facetings, and removes it if so.
fn filter_irc(vec: &Vec<Vec<(usize,usize)>>) -> Vec<usize> {
    let mut out = Vec::new(); // The indices of the facetings that aren't compounds.

    'a: for a in 0..vec.len() { // `a` is the index of the base set
        for b in 0..vec.len() { // `b` is the index of a potential subset of `a`
            if a == b {
                continue
            }
            if vec[b].len() > vec[a].len() { // A strict subset must be smaller than the base.
                continue
            }
            if vec[b][0] > vec[a][0] { // One of the subsets must contain the first facet.
                break
            }
            let mut i = 0;
            for f in &vec[a] { // Searches through `b` to see if all elements are in `a`.
                if &vec[b][i] > f {
                    continue
                }
                if &vec[b][i] < f {
                    break
                }
                i += 1;
                if i >= vec[b].len() { // We've found a subset.
                    continue 'a;
                }
            }
        }
        out.push(a)
    }
    out
}

fn faceting_subdim(
    rank: usize,
    plane: Subspace<f64>,
    points: Vec<PointOrd<f64>>,
    vertex_map: Vec<Vec<usize>>,
    min_edge_length: Option<f64>,
    max_edge_length: Option<f64>,
    max_per_hyperplane: Option<usize>,
	uniform: bool,
    noble_package: Option<(&Vec<Vec<usize>>, &Vec<usize>, usize)>,
	print_faceting_count: bool
) ->
    (Vec<(Ranks, Vec<(usize, usize)>)>, // Vec of facetings, along with the facet types of each of them
    Vec<usize>, // Counts of each hyperplane orbit
    Vec<Vec<Ranks>>, // Possible facets, these will be the possible ridges one dimension up
    HashMap<usize, (usize,usize)> // Map of compound facetings to their components.
) {
    let total_vert_count = points.len();

        let mut now = Instant::now();
    if rank == 2 {
        // The only faceting of a dyad is itself.
        // We distinguish between snub and non-snub edges.

        let mut snub = true;

        for row in &vertex_map {
            if row[0] == 1 {
                snub = false;
                break
            }
        }

        if snub {
            return (
                vec![(Abstract::dyad().ranks().clone(), vec![(0,0), (1,0)])],
                vec![1,1],
                vec![
                    vec![vec![
                        vec![].into(),
                        vec![
                            Element::new(vec![0].into(), vec![].into())
                            ].into(),
                        vec![
                            Element::new(vec![0].into(), vec![].into())
                            ].into(),
                    ].into()],
                    vec![vec![
                        vec![].into(),
                        vec![
                            Element::new(vec![0].into(), vec![].into())
                            ].into(),
                        vec![
                            Element::new(vec![1].into(), vec![].into())
                            ].into(),
                    ].into()]
                    ],
                    HashMap::new()
            )
        }
        else {
            return (
                vec![(Abstract::dyad().ranks().clone(), vec![(0,0)])],
                vec![2],
                vec![
                    vec![vec![
                        vec![].into(),
                        vec![
                            Element::new(vec![0].into(), vec![].into())
                            ].into(),
                        vec![
                            Element::new(vec![0].into(), vec![].into())
                            ].into(),
                    ].into()]
                    ],
                    HashMap::new()
            )
        }
    }
    let mut flat_points = Vec::new();
    for p in &points {
        flat_points.push(PointOrd::new(plane.flatten(&p.0)));
    }
    
    let mut vertex_orbits = Vec::new(); // Vec of orbits which are vecs of vertices.
    let mut orbit_of_vertex = vec![0; total_vert_count]; // For each vertex stores its orbit index.
    let mut checked_vertices = vec![false; total_vert_count]; // Stores whether we've already checked the vertex.

    let mut orbit_idx = 0;
    for v in 0..total_vert_count {
        if !checked_vertices[v] {
            // We found a new orbit of vertices.
            let mut new_orbit = Vec::new();
            for row in &vertex_map {
                // Find all vertices in the same orbit.
                let c = row[v];
                if !checked_vertices[c] {
                    new_orbit.push(c);
                    checked_vertices[c] = true;
                    orbit_of_vertex[c] = orbit_idx;
                }
            }
            vertex_orbits.push(new_orbit);
            orbit_idx += 1;
        }
    }

    let mut pair_orbits = Vec::new();
    let mut checked = vec![vec![false; total_vert_count]; total_vert_count];
    
    for orbit in vertex_orbits {
        let rep = orbit[0]; // We only need one representative per orbit.
        for vertex in rep+1..total_vert_count {
            if !checked[rep][vertex] {
                let edge_length = (&points[vertex].0-&points[rep].0).norm();
                if let Some(min) = min_edge_length {
                    if edge_length < min - f64::EPS {
                        continue
                    }
                }
                if let Some(max) = max_edge_length {
                    if edge_length > max + f64::EPS {
                        continue
                    }
                }
                let mut new_orbit = Vec::new();
                for row in &vertex_map {
                    let (a1, a2) = (row[rep], row[vertex]);
                    let c1 = a1.min(a2);
                    let c2 = a1.max(a2);
                    if !checked[c1][c2] {
                        new_orbit.push(vec![c1, c2]);
                        checked[c1][c2] = true;
                    }
                }
                pair_orbits.push(new_orbit);
            }
        }
    }

    // Enumerate hyperplanes
    let mut hyperplane_orbits = Vec::new();
    let mut checked = HashSet::<Vec<usize>>::new();
    let mut hyperplanes_vertices = Vec::new();

    let mut noble_map = HashMap::<Vec<usize>, usize>::new();
    let mut noble_counts = Vec::<usize>::new();
    let mut noble_muls = Vec::<usize>::new();

    for pair_orbit in pair_orbits {
        let rep = &pair_orbit[0];

        if rep[1]+rank-2 > points.len() {
            continue;
        }
        let mut new_vertices: Vec<usize> = (rep[1]+1..rep[1]+rank-2).collect();
        let mut update = 0;
        if rank > 3 {
            update = rank-4;
        }
        'b: loop {
            'c: loop {
                // WLOG checks if the vertices are all the right distance away from the first vertex.
                for (v_i, v) in new_vertices.iter().enumerate() {
                    let edge_length = (&points[*v].0-&points[rep[0]].0).norm();
                    if let Some(min) = min_edge_length {
                        if edge_length < min - f64::EPS {
                            update = v_i;
                            break 'c;
                        }
                    }
                    if let Some(max) = max_edge_length {
                        if edge_length > max + f64::EPS {
                            update = v_i;
                            break 'c;
                        }
                    }
                }
                // We start with a pair and add enough vertices to define a hyperplane.
                let mut tuple = rep.clone();
                tuple.append(&mut new_vertices.clone());

                let mut first_points = Vec::new();
                for v in tuple {
                    first_points.push(&flat_points[v].0);
                }

                let hyperplane = Subspace::from_points(first_points.clone().into_iter());
                if hyperplane.is_hyperplane() {

                    let mut hyperplane_vertices = Vec::new();
                    for (idx, v) in flat_points.iter().enumerate() {
                        if hyperplane.distance(&v.0) < f64::EPS {
                            hyperplane_vertices.push(idx);
                        }
                    }
                    hyperplane_vertices.sort_unstable();

                    // Check if the hyperplane has been found already.
                    if !checked.contains(&hyperplane_vertices) {
                        // If it's new, we add all the ones in its orbit.
                        let mut new_orbit = Vec::new();
                        let mut new_orbit_vertices = Vec::new();
                        for row in &vertex_map {
                            let mut new_hp_v = Vec::new();
                            for idx in &hyperplane_vertices {
                                new_hp_v.push(row[*idx]);
                            }
                            let new_hp_points = new_hp_v.iter().map(|x| &flat_points[*x].0);
                            let new_hp = Subspace::from_points(new_hp_points);

                            let mut sorted = new_hp_v.clone();
                            sorted.sort_unstable();

                            if !checked.contains(&sorted) {
                                checked.insert(sorted);
                                new_orbit.push(new_hp);
                                new_orbit_vertices.push(new_hp_v);
                            }
                        }

                        if let Some((full_vertex_map, global_v, count)) = noble_package {
                            let mut set = HashSet::new();

                            let mut global_hp_v = Vec::new();
                            for idx in &hyperplane_vertices {
                                global_hp_v.push(global_v[*idx]);
                            }
                            global_hp_v.sort_unstable();
                            
                            match noble_map.get(&global_hp_v) {
                                Some(idx) => {
                                    let mul = count * new_orbit.len() / noble_counts[*idx];
                                    noble_muls[*idx] += mul;
                                },
                                None => {
                                    for row in full_vertex_map {
                                        let mut new_hp_v = Vec::new();
                                        for idx in &hyperplane_vertices {
                                            new_hp_v.push(row[global_v[*idx]]);
                                        }
                                        
                                        let mut sorted = new_hp_v.clone();
                                        sorted.sort_unstable();
        
                                        set.insert(sorted.clone());
                                        noble_map.insert(sorted, noble_counts.len());
                                    }
        
                                    let mul = count * new_orbit.len() / set.len();
                                    noble_counts.push(set.len());
                                    noble_muls.push(mul);
                                },
                            }
                        }

                        hyperplane_orbits.push(new_orbit);
                        hyperplanes_vertices.push(new_orbit_vertices);
                    }
                }
                break;
            }
            if rank <= 3 {
                break;
            }
            loop { // Increment new_vertices.
                if new_vertices[update] == total_vert_count + update - rank + 3 {
                    if update < 1 {
                        break 'b;
                    }
                    else {
                        update -= 1;
                    }
                } else {
                    new_vertices[update] += 1;
                    for i in update+1..rank-3 {
                        new_vertices[i] = new_vertices[i-1]+1;
                    }
                    update = rank-4;
                    break;
                }
            }
        }
    }
    // Filter the invalid hyperplanes if noble faceting.
    if let Some((_, global_v, _)) = noble_package {
        let mut new_hyperplane_orbits = Vec::new();
        let mut new_hyperplanes_vertices = Vec::new();

        for (idx, orbit) in hyperplanes_vertices.iter().enumerate() {
            let mut global_hp_v: Vec<usize> = orbit[0].clone().iter().map(|x| global_v[*x]).collect();
            global_hp_v.sort_unstable();
            if noble_muls[*noble_map.get(&global_hp_v).unwrap()] >= 2 {
                new_hyperplane_orbits.push(hyperplane_orbits[idx].clone());
                new_hyperplanes_vertices.push(orbit.clone());
            }
        }

        hyperplane_orbits = new_hyperplane_orbits;
        hyperplanes_vertices = new_hyperplanes_vertices;
    }

    // Facet the hyperplanes
    let mut possible_facets = Vec::new();
    let mut possible_facets_global: Vec<Vec<(Ranks, Vec<(usize,usize)>)>> = Vec::new(); // copy of above but with semi-global vertex indices
    let mut compound_facets: Vec<HashMap<usize, (usize,usize)>> = Vec::new();
    let mut ridges: Vec<Vec<Vec<Ranks>>> = Vec::new();
    let mut ff_counts = Vec::new();

    for (i, orbit) in hyperplane_orbits.iter().enumerate() {
        let (hp, hp_v) = (orbit[0].clone(), hyperplanes_vertices[i][0].clone());
        let mut stabilizer = Vec::new();
        for row in &vertex_map {
            let mut slice = Vec::new();
            for v in &hp_v {
                slice.push(row[*v]);
            }
            let mut slice_sorted = slice.clone();
            slice_sorted.sort_unstable();

            if slice_sorted == hp_v {
                stabilizer.push(slice.clone());
            }
        }

        // Converts global vertex indices to local ones.
        let mut map_back = BTreeMap::new();
        for (idx, el) in stabilizer[0].iter().enumerate() {
            map_back.insert(*el, idx);
        }
        
        let mut new_stabilizer = stabilizer.clone();

        for a in 0..stabilizer.len() {
            for b in 0..stabilizer[a].len() {
                new_stabilizer[a][b] = *map_back.get(&stabilizer[a][b]).unwrap();
            }
        }

        let mut points = Vec::new();
        for v in &hp_v {
            points.push(flat_points[*v].clone());
        }

        let (possible_facets_row, ff_counts_row, ridges_row, compound_facets_row) =
            faceting_subdim(rank-1, hp, points, new_stabilizer.clone(), min_edge_length, max_edge_length, max_per_hyperplane, uniform, None, false);

        let mut possible_facets_global_row = Vec::new();
        for f in &possible_facets_row {
            let mut new_f = f.clone();
            let mut new_edges = ElementList::new();
            for v in f.0[2].clone() {
                // Converts indices back to semi-global
                let mut new_edge = Element::new(vec![].into(), vec![].into());
                for s in v.subs {
                    new_edge.subs.push(hp_v[s]);
                }
                new_edges.push(new_edge);
            }
            new_f.0[2] = new_edges;

            possible_facets_global_row.push(new_f);
        }
        possible_facets.push(possible_facets_row);
        possible_facets_global.push(possible_facets_global_row);
        compound_facets.push(compound_facets_row);
        ridges.push(ridges_row);
        ff_counts.push(ff_counts_row);
    }

    let mut ridge_idx_orbits = Vec::new();
    let mut ridge_orbits = HashMap::new();
    let mut ridge_counts = Vec::new(); // Counts the number of ridges in each orbit
    let mut orbit_idx = 0;

    let mut hp_i = 0; // idk why i have to do this, thanks rust
    for ridges_row in ridges {
        let mut r_i_o_row = Vec::new();

        for ridges_row_row in ridges_row {
            let mut r_i_o_row_row = Vec::new();

            for mut ridge in ridges_row_row {
                // goes through all the ridges

                // globalize
                let mut new_list = ElementList::new();
                for i in 0..ridge[2].len() {
                    let mut new = Element::new(Subelements::new(), Superelements::new());
                    for sub in &ridge[2][i].subs {
                        new.subs.push(hyperplanes_vertices[hp_i][0][*sub])
                    }
                    new_list.push(new);
                }
                ridge[2] = new_list;

                ridge.element_sort_strong();

                match ridge_orbits.get(&ridge) {
                    Some(idx) => {
                        // writes the orbit index at the ridge index
                        r_i_o_row_row.push(*idx);
                    }
                    None => {
                        // adds all ridges with the same orbit to the map
                        let mut count = 0;
                        for row in &vertex_map {
                            let mut new_ridge = ridge.clone();

                            let mut new_list = ElementList::new();
                            for i in 0..new_ridge[2].len() {
                                let mut new = Element::new(Subelements::new(), Superelements::new());
                                for sub in &ridge[2][i].subs {
                                    new.subs.push(row[*sub])
                                }
                                new_list.push(new);
                            }
                            new_ridge[2] = new_list;

                            new_ridge.element_sort_strong();

                            if ridge_orbits.get(&new_ridge).is_none() {
                                ridge_orbits.insert(new_ridge, orbit_idx);
                                count += 1;
                            }
                        }
                        r_i_o_row_row.push(orbit_idx);
                        ridge_counts.push(count);
                        orbit_idx += 1;
                    }
                }
            }
            r_i_o_row.push(r_i_o_row_row);
        }
        ridge_idx_orbits.push(r_i_o_row);
        hp_i += 1;
    }

    let mut f_counts = Vec::new();
    for orbit in hyperplane_orbits {
        f_counts.push(orbit.len());
    }

    // Actually do the faceting
    let mut ridge_muls = Vec::new();
    let mut ones = vec![Vec::<(usize, usize)>::new(); ridge_counts.len()];

    for (hp, list) in possible_facets.iter().enumerate() {
        let mut ridge_muls_hp = Vec::new();
        for (f, _) in list.iter().enumerate() {
            let mut ridge_muls_facet = vec![0; ridge_counts.len()];

            let f_count = f_counts[hp];

            let ridge_idxs_local = &possible_facets[hp][f].1;
            for ridge_idx in ridge_idxs_local {
                let ridge_orbit = ridge_idx_orbits[hp][ridge_idx.0][ridge_idx.1];
                let ridge_count = ff_counts[hp][ridge_idx.0];
                let total_ridge_count = ridge_counts[ridge_orbit];
                let mul = f_count * ridge_count / total_ridge_count;

                if mul == 1 {
                    ones[ridge_orbit].push((hp, f));
                }

                ridge_muls_facet[ridge_orbit] = mul;
            }

            ridge_muls_hp.push(ridge_muls_facet);
        }
        ridge_muls.push(ridge_muls_hp);
    }

    let mut output = Vec::new();
    let mut output_facets = Vec::new();

    let mut facets_queue = VecDeque::<(
        Vec<(usize, usize)>, // list of facets
        usize, // min hyperplane
        Vec<usize> // cached ridge muls
    )>::new();

    for (hp, list) in possible_facets.iter().enumerate() {
        for f in 0..list.len() {
            facets_queue.push_back((
                vec![(hp, f)],
                hp,
                vec![0; ridge_counts.len()]
            ));
        }
    }

	let mut skipped = 0;
    'l: while let Some((facets, min_hp, cached_ridge_muls)) = facets_queue.pop_back() {
        if uniform {
            if now.elapsed().as_millis() > DELAY && print_faceting_count {
                print!("{}", CL);
                print!("{:.115}", format!("{} facets found, {} skipped, {:?}", output.len(), skipped, facets));
                std::io::stdout().flush().unwrap();
                now = Instant::now();
            }
        } else {
            if now.elapsed().as_millis() > DELAY && print_faceting_count {
                print!("{}", CL);
                print!("{:.115}", format!("{} facets found, {:?}", output.len(), facets));
                std::io::stdout().flush().unwrap();
                now = Instant::now();
            }
        }
        
        let mut new_ridge_muls = cached_ridge_muls.clone();

        let last_facet = facets.last().unwrap();

        'a: loop {
            let hp = last_facet.0;
            let f = last_facet.1;

            let ridge_idxs_local = &possible_facets[hp][f].1;
            for ridge_idx in ridge_idxs_local {
                let ridge_orbit = ridge_idx_orbits[hp][ridge_idx.0][ridge_idx.1];
                let mul = ridge_muls[hp][f][ridge_orbit];

                new_ridge_muls[ridge_orbit] += mul;
                if new_ridge_muls[ridge_orbit] > 2 {
                    break 'a;
                }
            }
            break;
        }
        let mut valid = 0; // 0: valid, 1: exotic, 2: incomplete
        for r in &new_ridge_muls {
            if *r > 2 {
                valid = 1;
                break
            }
            if *r == 1 {
                valid = 2;
            }
        }
        match valid {
            0 => {
                // Split compound facets into their components.
                let mut new_facets = Vec::new();

                for (hp, idx) in &facets {
                    let mut all_components = Vec::<usize>::new();
                    let mut queue = VecDeque::new();
                    queue.push_back(*idx);
                    while let Some(next) = queue.pop_front() {
                        if let Some(components) = compound_facets[*hp].get(&next) {
                            queue.push_back(components.0);
                            queue.push_back(components.1);
                        } else {
                            all_components.push(next);
                        }
                    }
                    for component in all_components {
                        new_facets.push((*hp, component));
                    }
                }

                // Output the faceted polytope. We will build it from the set of its facets.

                let mut facet_set = HashSet::new();
                for facet_orbit in &new_facets {
                    let facet = &possible_facets_global[facet_orbit.0][facet_orbit.1].0;
                    let facet_local = &possible_facets[facet_orbit.0][facet_orbit.1].0;
                    for row in &vertex_map {
                        let mut new_facet = facet.clone();
                            
                        let mut new_list = ElementList::new();
                        for i in 0..facet[2].len() {
                            let mut new = Element::new(Subelements::new(), Superelements::new());
                            for sub in &facet[2][i].subs {
                                new.subs.push(row[*sub])
                            }
                            new_list.push(new);
                        }
                        new_facet[2] = new_list;

                        new_facet.element_sort_strong_with_local(facet_local);
                        facet_set.insert(new_facet);
                    }
                }

                let mut facet_vec = Vec::from_iter(facet_set.clone());
                let mut facet_vec2 = Vec::from_iter(facet_set);

                let mut ranks = Ranks::new();
                ranks.push(vec![Element::new(vec![].into(), vec![].into())].into()); // nullitope
                ranks.push(vec![Element::new(vec![0].into(), vec![].into()); total_vert_count].into()); // vertices
				
                let mut ranks2 = Ranks::new();
                ranks2.push(vec![Element::new(vec![].into(), vec![].into())].into()); // nullitope

                let mut to_new_idx = HashMap::new();
                let mut to_old_idx = Vec::new();
                let mut idx = 0;
                if uniform {
                    for i in 0..facet_vec2.len() {
                        let mut new_list = ElementList::new();
                        for j in 0..facet_vec2[i][2].len() {
                            let mut new = Element::new(Subelements::new(), Superelements::new());
                            for sub in facet_vec2[i][2][j].subs.clone() {
                                if to_new_idx.get(&sub).is_none() {
                                    to_new_idx.insert(sub, idx);
                                    to_old_idx.push(sub);
                                    idx += 1;
                                }
                                new.subs.push(*to_new_idx.get(&sub).unwrap())
                            }
                            new_list.push(new);
                        }
                        facet_vec2[i][2] = new_list;
                    }
                    let mut new_rank = ElementList::new();
                    for _i in 0..idx {
                        new_rank.push(Element::new(vec![0].into(), vec![].into()));
                    }
                    ranks2.push(new_rank);
                }

                for r in 2..rank-1 { // edges and up
                    let mut subs_to_idx = HashMap::new();
                    let mut idx_to_subs = Vec::new();
                    let mut idx = 0;

                    for facet in &facet_vec {
                        let els = &facet[r];
                        for el in els {
                            if subs_to_idx.get(&el.subs).is_none() {
                                subs_to_idx.insert(el.subs.clone(), idx);
                                idx_to_subs.push(el.subs.clone());
                                idx += 1;
                            }
                        }
                    }
                    for i in 0..facet_vec.len() {
                        let mut new_list = ElementList::new();
                        for j in 0..facet_vec[i][r+1].len() {
                            let mut new = Element::new(Subelements::new(), Superelements::new());
                            for sub in &facet_vec[i][r+1][j].subs {
                                let sub_subs = &facet_vec[i][r][*sub].subs;
                                new.subs.push(*subs_to_idx.get(sub_subs).unwrap())
                            }
                            new_list.push(new);
                        }
                        facet_vec[i][r+1] = new_list;
                    }

                    let mut new_rank = ElementList::new();
                    for el in idx_to_subs {
                        new_rank.push(Element::new(el, vec![].into()));
                    }
                    ranks.push(new_rank);
					
					if uniform {
						let mut subs_to_idx = HashMap::new();
						let mut idx_to_subs = Vec::new();
						let mut idx = 0;
						for facet in &facet_vec2 {
							let els = &facet[r];
							for el in els {
								if subs_to_idx.get(&el.subs).is_none() {
									subs_to_idx.insert(el.subs.clone(), idx);
									idx_to_subs.push(el.subs.clone());
									idx += 1;
								}
							}
						}
						for i in 0..facet_vec2.len() {
							let mut new_list = ElementList::new();
							for j in 0..facet_vec2[i][r+1].len() {
								let mut new = Element::new(Subelements::new(), Superelements::new());
								for sub in &facet_vec2[i][r+1][j].subs {
									let sub_subs = &facet_vec2[i][r][*sub].subs;
									new.subs.push(*subs_to_idx.get(sub_subs).unwrap())
								}
								new_list.push(new);
							}
							facet_vec2[i][r+1] = new_list;
						}

						let mut new_rank = ElementList::new();
						for el in idx_to_subs {
							new_rank.push(Element::new(el, vec![].into()));
						}
						ranks2.push(new_rank);
					}
                }
                let mut new_rank = ElementList::new();
                let mut set = HashSet::new();

                for f_i in 0..facet_vec.len() {
                    facet_vec[f_i][rank-1][0].subs.sort();
                    let subs = facet_vec[f_i][rank-1][0].subs.clone();
                    if !set.contains(&subs) {
                        new_rank.push(Element::new(subs.clone(), Superelements::new()));
                        set.insert(subs);
                    }
                }
                let n_r_len = new_rank.len();
                ranks.push(new_rank); // facets

                ranks.push(vec![Element::new(Subelements::from_iter(0..n_r_len), Superelements::new())].into()); // body
				
				if uniform {
					let mut new_rank = ElementList::new();
					let mut set = HashSet::new();

					for f_i in 0..facet_vec2.len() {
						facet_vec2[f_i][rank-1][0].subs.sort();
						let subs = facet_vec2[f_i][rank-1][0].subs.clone();
						if !set.contains(&subs) {
							new_rank.push(Element::new(subs.clone(), Superelements::new()));
							set.insert(subs);
						}
					}
					let n_r_len = new_rank.len();
					ranks2.push(new_rank); // facets

					ranks2.push(vec![Element::new(Subelements::from_iter(0..n_r_len), Superelements::new())].into()); // body
				}

                if uniform {
                    unsafe {
                        let mut builder = AbstractBuilder::new();
                        for rank in ranks2 {
                            builder.push_empty();
                            for el in rank {
                                builder.push_subs(el.subs);
                            }
                        }
            
                        if builder.ranks().is_dyadic().is_ok() {
                            let abs = builder.build();
                            let mut new_vertices = Vec::new();
                            for i in to_old_idx {
                                new_vertices.push(flat_points[i].0.clone());
                            }

                            let mut poly = Concrete {
                                vertices: new_vertices,
                                abs: abs.clone(),
                            };
                            poly.recenter();
                            
                            let amount = poly.element_types()[1].len();
                            
                            if amount <= 1 {
                                output.push((ranks, new_facets.clone()));
                                output_facets.push(new_facets.clone());
                            } else {
								poly.element_sort();
								let components = poly.defiss();
								let mut isogonal = true;
								for component in components {
									if component.element_types()[1].len() > 1 {
										isogonal = false;
										break;
									}
								}
								if isogonal {
									output.push((ranks, new_facets.clone()));
									output_facets.push(new_facets.clone());
								} else {
									skipped += 1;
								}
                            }
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    output.push((ranks, new_facets.clone()));
                    output_facets.push(new_facets.clone());
                }

                if let Some(max) = max_per_hyperplane {
                    if output.len() + skipped >= max {
                        break 'l;
                    }
                }

                if noble_package.is_none() {
                    let mut used_hps = HashSet::new();
                    for facet in facets.iter().skip(1) {
                        used_hps.insert(facet.0);
                    }
                    for (hp, list) in possible_facets.iter().enumerate().skip(min_hp+1) {
                        if !used_hps.contains(&hp) {
                            for f in 0..list.len() {
                                let mut new_facets = facets.clone();
                                new_facets.push((hp, f));
                                facets_queue.push_back((new_facets, hp, new_ridge_muls.clone()));
                            }
                        }
                    }
                }
            }
            1 => {}
            2 => {
                let mut used_hps = HashSet::new();
                for facet in facets.iter().skip(1) {
                    used_hps.insert(facet.0);
                }
                for (idx, mul) in new_ridge_muls.iter().enumerate() {
                    if *mul == 1 {
                        for facet in ones[idx]
                            .iter()
                            .skip(binary(&ones[idx], min_hp))
                        {
                            if !used_hps.contains(&facet.0) {
                                let mut new_facets = facets.clone();
                                new_facets.push(*facet);
                                facets_queue.push_back((new_facets, min_hp, new_ridge_muls.clone()));
                            }
                        }
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    output.sort_by(|a,b| a.1.cmp(&b.1));
    output_facets.sort_unstable();

    let mut output_ridges = Vec::new();
    for i in possible_facets_global {
        let mut a = Vec::new();
        for j in i {
            a.push(j.0);
        }
        output_ridges.push(a);
    }

    return (output, f_counts, output_ridges, label_irc(&output_facets))
}

impl Concrete {
    /// Enumerates the facetings of a polytope under a provided symmetry group or vertex map.
    /// If the symmetry group is not provided, it uses the full symmetry of the polytope.
    pub fn faceting(
        &mut self,
        vertices: Vec<Point<f64>>,
        symmetry: GroupEnum,
        any_single_edge_length: bool,
        mut min_edge_length: Option<f64>,
        mut max_edge_length: Option<f64>,
        min_inradius: Option<f64>,
        max_inradius: Option<f64>,
        exclude_hemis: bool,
        only_below_vertex: bool,
        noble: Option<usize>,
        max_per_hyperplane: Option<usize>,
		uniform: bool,
        include_compounds: bool,
        mark_fissary: bool,
        label_facets: bool,
        save: bool,
        save_facets: bool,
        save_to_file: bool,
        file_path: String
    ) -> Vec<(Concrete, Option<String>)> {
        let rank = self.rank();
        let mut now = Instant::now();

        if rank < 4 {
            println!("\nFaceting polytopes of rank less than 3 is not supported!\n");
            return Vec::new()
        }

        let mut vertices_ord = Vec::<PointOrd<f64>>::new();
        for v in &vertices {
            vertices_ord.push(PointOrd::new(v.clone()));
        }

        let vertex_map = match symmetry {
            GroupEnum::ConcGroup(group) => {
                println!("\nComputing vertex map...");
                self.get_vertex_map(group)
            },
            GroupEnum::VertexMap(a) => a,
            GroupEnum::Chiral(chiral) => {
                if chiral {
                    println!("\nComputing rotation symmetry group...");
                    let g = self.get_rotation_group().unwrap();
                    println!("Rotation symmetry order {}", g.0.count());
                    g.1
                }
                else {
                    println!("\nComputing symmetry group...");
                    let g = self.get_symmetry_group().unwrap();
                    println!("Symmetry order {}", g.0.count());
                    g.1
                }
            },
        };

        let mut output = Vec::new();

        println!("\nMatching vertices...");

        // Checking every r-tuple of vertices would take too long, so we put pairs into orbits first to reduce the number.
        // I don't think we need to store the whole orbits at this point, but they might be useful if we want to improve the algorithm.
        let mut vertex_orbits = Vec::new(); // Vec of orbits which are vecs of vertices.
        let mut orbit_of_vertex = vec![0; vertices.len()]; // For each vertex stores its orbit index.
        let mut checked_vertices = vec![false; vertices.len()]; // Stores whether we've already checked the vertex.

        let mut orbit_idx = 0;
        for v in 0..vertices.len() {
            if !checked_vertices[v] {
                // We found a new orbit of vertices.
                let mut new_orbit = Vec::new();
                for row in &vertex_map {
                    // Find all vertices in the same orbit.
                    let c = row[v];
                    if !checked_vertices[c] {
                        new_orbit.push(c);
                        checked_vertices[c] = true;
                        orbit_of_vertex[c] = orbit_idx;
                    }
                }
                vertex_orbits.push(new_orbit);
                orbit_idx += 1;
            }
        }

        println!("{} vertices in {} orbit{}", vertices.len(), orbit_idx, if orbit_idx == 1 {""} else {"s"});

        let mut possible_lengths_set = BTreeSet::<OrderedFloat<f64>>::new();
        let mut possible_lengths = Vec::new();

        if any_single_edge_length {
            println!("\nComputing edge lengths...");

            for orbit in &vertex_orbits {
                let rep = orbit[0];
                for i in rep+1..vertices.len() {
                    possible_lengths_set.insert(OrderedFloat((vertices[rep].clone() - vertices[i].clone()).norm()));
                }
            }
            let mut possible_lengths_ordf: Vec<&OrderedFloat<f64>> = possible_lengths_set.iter().collect();
            possible_lengths_ordf.sort_unstable();

            if possible_lengths_ordf.len() > 0 {
                possible_lengths.push(possible_lengths_ordf[0].0);
            }
            for idx in 0..possible_lengths_ordf.len()-1 {
                let len1 = possible_lengths_ordf[idx].0;
                let len2 = possible_lengths_ordf[idx+1].0;
                if len2-len1 > f64::EPS {
                    possible_lengths.push(len2);
                }
            }

            println!("Found {} edge lengths: {:?}", possible_lengths.len(), possible_lengths);
        }
        let mut edge_length_idx = 0;
        
        loop {
            if any_single_edge_length {
                let edge_length = possible_lengths[edge_length_idx];
                min_edge_length = Some(edge_length);
                max_edge_length = Some(edge_length);
                println!("\nChecking edge length {} ({}/{})", edge_length, edge_length_idx+1, possible_lengths.len());
            }

            println!("\nEnumerating hyperplanes...");

            let mut hyperplane_orbits = Vec::new();

            if only_below_vertex {
                for v_orbit in &vertex_orbits {
                    let mut map = BTreeMap::<OrderedFloat<f64>, Vec<usize>>::new();
                    let rep = v_orbit[0];
                    let point = &vertices[rep];

                    for (idx, vertex) in vertices.iter().enumerate() {
                        let dot = OrderedFloat(vertex.dot(point));
                        if let Some(list) = map.get_mut(&dot) {
                            list.push(idx);
                        } else {
                            map.insert(dot, vec![idx]);
                        }
                    }
                    
                    let mut checked = HashSet::new();

                    let mut dbg_count: u64 = 0;

                    'd: for (_dot, l) in &map {
                        let mut list = l.clone();
                        list.sort_unstable();

                        if now.elapsed().as_millis() > DELAY {
                            print!("{}loop {}, verts {:?}", CL, dbg_count, list);
                            std::io::stdout().flush().unwrap();
                            now = Instant::now();
                        }
                        dbg_count += 1;

                        // WLOG checks if the vertices are all the right distance away from the first vertex.
                        for v in &list[1..] {
                            let edge_length = (&vertices[*v]-&vertices[list[0]]).norm();
                            if let Some(min) = min_edge_length {
                                if edge_length < min - f64::EPS {
                                    continue 'd;
                                }
                            }
                            if let Some(max) = max_edge_length {
                                if edge_length > max + f64::EPS {
                                    continue 'd;
                                }
                            }
                        }

                        // We define a hyperplane from the list of vertices.
                        let points = list.iter().map(|x| &vertices[*x]);

                        let hyperplane = Subspace::from_points(points);

                        if hyperplane.is_hyperplane() {
                            let inradius = hyperplane.distance(&Point::zeros(self.dim().unwrap()));
                            if let Some(min) = min_inradius {
                                if inradius < min - f64::EPS {
                                    continue
                                }
                            }
                            if let Some(max) = max_inradius {
                                if inradius > max + f64::EPS {
                                    continue
                                }
                            }
                            if exclude_hemis {
                                if inradius.abs() < f64::EPS {
                                    continue
                                }
                            }

                            let mut hyperplane_vertices = Vec::new();
                            for (idx, v) in vertices.iter().enumerate() {
                                if hyperplane.distance(&v) < f64::EPS {
                                    hyperplane_vertices.push(idx);
                                }
                            }
                            hyperplane_vertices.sort_unstable();

                            // Check if the hyperplane has been found already.
                            let mut is_new = true;
                            let mut counting = HashSet::<Vec<usize>>::new();
                            for row in &vertex_map {
                                let mut new_hp_v = Vec::new();
                                for idx in &hyperplane_vertices {
                                    new_hp_v.push(row[*idx]);
                                }
                                new_hp_v.sort_unstable();

                                if checked.contains(&new_hp_v) {
                                    is_new = false;
                                    break
                                }

                                counting.insert(new_hp_v);
                            }
                            if is_new {
                                checked.insert(hyperplane_vertices.clone());
                                hyperplane_orbits.push((hyperplane, hyperplane_vertices, counting.len()));
                            }
                        }
                    }
                }
            }
            else {

                // Enumerate edges

                let mut pair_orbits = Vec::new();
                let mut checked = vec![vec![false; vertices.len()]; vertices.len()];
                
                for orbit in &vertex_orbits {
                    let rep = orbit[0]; // We only need one representative per orbit.
                    for vertex in rep+1..vertices.len() {
                        if now.elapsed().as_millis() > DELAY {
                            print!("{}{} edge orbits, verts [{}, {}]", CL, pair_orbits.len(), rep, vertex);
                            std::io::stdout().flush().unwrap();
                            now = Instant::now();
                        }

                        if !checked[rep][vertex] {
                            let edge_length = (&vertices[vertex]-&vertices[rep]).norm();
                            if let Some(min) = min_edge_length {
                                if edge_length < min - f64::EPS {
                                    continue;
                                }
                            }
                            if let Some(max) = max_edge_length {
                                if edge_length > max + f64::EPS {
                                    continue;
                                }
                            }
                            let mut new_orbit = Vec::new();
                            for row in &vertex_map {
                                let (a1, a2) = (row[rep], row[vertex]);
                                let c1 = a1.min(a2);
                                let c2 = a1.max(a2);
                                
                                if !checked[c1][c2] {
                                    new_orbit.push(vec![c1, c2]);
                                    checked[c1][c2] = true;
                                }
                            }
                            pair_orbits.push(new_orbit);
                        }
                    }
                }

                println!("{}{} edge orbit{}", CL, pair_orbits.len(), if pair_orbits.len() == 1 {""} else {"s"});

                // Enumerate subspaces between lines and hyperplanes

                let mut tuple_orbits: Vec<Vec<usize>> = pair_orbits.iter().map(|orbit| orbit[0].clone()).collect();
                for number in 3..rank-1 {
                    let mut checked = HashSet::new();
                    let mut new_tuple_orbits = Vec::new();

                    for tuple in tuple_orbits {
                        for new_vertex in tuple[tuple.len()-1]..vertices.len() {
                            if now.elapsed().as_millis() > DELAY {
                                print!("{}{} {}-plane orbits, verts {:?}", CL, new_tuple_orbits.len(), number-1, tuple);
                                std::io::stdout().flush().unwrap();
                                now = Instant::now();
                            }

                            let mut wrong_edge = false;

                            let edge_length = (&vertices[tuple[0]]-&vertices[new_vertex]).norm();
                            if let Some(min) = min_edge_length {
                                if edge_length < min - f64::EPS {
                                    wrong_edge = true;
                                }
                            }
                            if let Some(max) = max_edge_length {
                                if edge_length > max + f64::EPS {
                                    wrong_edge = true;
                                }
                            }
                            if wrong_edge {
                                continue;
                            }

                            let mut new_tuple = tuple.clone();
                            new_tuple.push(new_vertex);

                            let mut already_seen = false;
                            for row in &vertex_map {
                                let mut moved: Vec<usize> = new_tuple.iter().map(|x| row[*x]).collect();
                                moved.sort_unstable();

                                if checked.contains(&moved) {
                                    already_seen = true;
                                    break;
                                }
                            }
                            if already_seen {
                                continue;
                            }

                            new_tuple.sort_unstable();

                            let subspace = Subspace::from_points(new_tuple.iter().map(|x| &vertices[*x]));
                            if subspace.rank() == number-1 {
                                new_tuple_orbits.push(new_tuple.clone());
                            }

                            checked.insert(new_tuple);
                        }
                    }
                    println!("{}{} {}-plane orbit{}", CL, new_tuple_orbits.len(), number-1, if new_tuple_orbits.len() == 1 {""} else {"s"});
                    tuple_orbits = new_tuple_orbits.iter().map(|x| x.clone()).collect();
                }

                // Enumerate hyperplanes
                let mut checked = HashSet::new();

                for rep in tuple_orbits {
                    let last_vert = rep[rep.len()-1];

                    for new_vertex in last_vert+1..vertices.len() {
                        let mut tuple = rep.clone();
                        tuple.push(new_vertex);

                        if now.elapsed().as_millis() > DELAY {
                            print!("{}{} hyperplane orbits, verts {:?}", CL, hyperplane_orbits.len(), tuple);
                            std::io::stdout().flush().unwrap();
                            now = Instant::now();
                        }

                        let edge_length = (&vertices[new_vertex]-&vertices[rep[0]]).norm();
                        if let Some(min) = min_edge_length {
                            if edge_length < min - f64::EPS {
                                continue;
                            }
                        }
                        if let Some(max) = max_edge_length {
                            if edge_length > max + f64::EPS {
                                continue;
                            }
                        }

                        let mut points = Vec::new();
                        for v in tuple {
                            points.push(vertices[v].clone());
                        }

                        let hyperplane = Subspace::from_points(points.iter());

                        if hyperplane.is_hyperplane() {
                            let inradius = hyperplane.distance(&Point::zeros(self.dim().unwrap()));
                            if let Some(min) = min_inradius {
                                if inradius < min - f64::EPS {
                                    break
                                }
                            }
                            if let Some(max) = max_inradius {
                                if inradius > max + f64::EPS {
                                    break
                                }
                            }
                            if exclude_hemis {
                                if inradius.abs() < f64::EPS {
                                    break
                                }
                            }

                            let mut hyperplane_vertices = Vec::new();
                            for (idx, v) in vertices.iter().enumerate() {
                                if hyperplane.distance(&v) < f64::EPS {
                                    hyperplane_vertices.push(idx);
                                }
                            }
                            hyperplane_vertices.sort_unstable();

                            // Check if the hyperplane has been found already.
                            let mut is_new = true;
                            let mut counting = HashSet::<Vec<usize>>::new();
                            for row in &vertex_map {
                                let mut new_hp_v = Vec::new();
                                for idx in &hyperplane_vertices {
                                    new_hp_v.push(row[*idx]);
                                }
                                new_hp_v.sort_unstable();

                                if checked.contains(&new_hp_v) {
                                    is_new = false;
                                    break;
                                }

                                counting.insert(new_hp_v);
                            }
                            if is_new {
                                checked.insert(hyperplane_vertices.clone());
                                hyperplane_orbits.push((hyperplane, hyperplane_vertices, counting.len()));
                            }
                        }
                    }
                }
            }

            let mut sum: u64 = 0;
            let mut f_counts = Vec::new();
            for orbit in &hyperplane_orbits {
                let count = orbit.2;
                f_counts.push(count);
                sum += count as u64;
            }

            println!("{}{} hyperplanes in {} orbit{}", CL, sum, hyperplane_orbits.len(), if hyperplane_orbits.len() == 1 {""} else {"s"});

            println!("\nFaceting hyperplanes...");

            // Facet the hyperplanes
            let mut possible_facets = Vec::new();
            let mut possible_facets_global: Vec<Vec<(Ranks, Vec<(usize,usize)>)>> = Vec::new(); // copy of above but with global vertex indices
            let mut compound_facets: Vec<HashMap<usize, (usize,usize)>> = Vec::new();
            let mut ridges: Vec<Vec<Vec<Ranks>>> = Vec::new();
            let mut ff_counts = Vec::new();

            for (idx, orbit) in hyperplane_orbits.iter().enumerate() {
                let (hp, hp_v) = (orbit.0.clone(), orbit.1.clone());
                let mut stabilizer = Vec::new();
                for row in &vertex_map {
                    let mut slice = Vec::new();
                    for v in &hp_v {
                        slice.push(row[*v]);
                    }
                    let mut slice_sorted = slice.clone();
                    slice_sorted.sort_unstable();

                    if slice_sorted == hp_v {
                        stabilizer.push(slice.clone());
                    }
                }

                // Converts global vertex indices to local ones.
                let mut map_back = BTreeMap::new();
                for (idx, el) in stabilizer[0].iter().enumerate() {
                    map_back.insert(*el, idx);
                }
                let mut new_stabilizer = stabilizer.clone();
        
                for a in 0..stabilizer.len() {
                    for b in 0..stabilizer[a].len() {
                        new_stabilizer[a][b] = *map_back.get(&stabilizer[a][b]).unwrap();
                    }
                }
                
                let mut points = Vec::new();
                for v in &hp_v {
                    points.push(vertices_ord[*v].clone());
                }

                let noble_package = if noble == Some(1) {
                    Some((&vertex_map, &hp_v, orbit.2))
                } else {
                    None
                };

                let (possible_facets_row, ff_counts_row, ridges_row, compound_facets_row) =
                    faceting_subdim(rank-1, hp, points, new_stabilizer, min_edge_length, max_edge_length, max_per_hyperplane, uniform, noble_package, true);

                let mut possible_facets_global_row = Vec::new();
                for f in &possible_facets_row {
                    let mut new_f = f.clone();
                    let mut new_edges = ElementList::new();
                    for v in f.0[2].clone() {
                        // Converts indices back to global
                        let mut new_edge = Element::new(vec![].into(), vec![].into());
                        for s in v.subs {
                            new_edge.subs.push(hp_v[s]);
                        }
                        new_edges.push(new_edge);
                    }
                    new_f.0[2] = new_edges;

                    possible_facets_global_row.push(new_f);
                }
                possible_facets.push(possible_facets_row.clone());
                possible_facets_global.push(possible_facets_global_row);
                compound_facets.push(compound_facets_row);
                ridges.push(ridges_row);
                ff_counts.push(ff_counts_row);

                println!("{}{}: {} facets, {} verts, {} copies", CL, idx, possible_facets_row.len(), hp_v.len(), orbit.2);
                std::io::stdout().flush().unwrap();
            }

            println!("\nComputing ridges...");

            let mut ridge_idx_orbits = Vec::new();
            let mut ridge_orbits = HashMap::new();
            let mut ridge_counts = Vec::new(); // Counts the number of ridges in each orbit
            let mut orbit_idx = 0;

            for (hp_i, ridges_row) in ridges.iter_mut().enumerate() {
                let mut r_i_o_row = Vec::new();

                for ridges_row_row in ridges_row {
                    let mut r_i_o_row_row = Vec::new();

                    for ridge in ridges_row_row {
                        // goes through all the ridges

                        // globalize
                        let mut new_list = ElementList::new();
                        for i in 0..ridge[2].len() {
                            let mut new = Element::new(Subelements::new(), Superelements::new());
                            for sub in &ridge[2][i].subs {
                                new.subs.push(hyperplane_orbits[hp_i].1[*sub])
                            }
                            new_list.push(new);
                        }
                        ridge[2] = new_list;

                        ridge.element_sort_strong();

                        /*
                        // look for possible disentanglement
                        let mut disentangled = None;

                        let mut ridge_vertices_idx = HashSet::new();
                        
                        for edge in &ridge[2] {
                            for sub in &edge.subs {
                                ridge_vertices_idx.insert(*sub);
                            }
                        }

                        let mut ridge_vertices = Vec::new();

                        for idx in &ridge_vertices_idx {
                            ridge_vertices.push(vertices[*idx].clone());
                        }

                        let subspace = Subspace::from_points(ridge_vertices.iter());
                        let mut all_vertices_idx = HashSet::new();

                        for (i, vertex) in vertices.iter().enumerate() {
                            if subspace.distance(&vertex) < f64::EPS {
                                all_vertices_idx.insert(i);
                            }
                        }

                        if all_vertices_idx.len() > ridge_vertices_idx.len() {
                            'vmap: for row in vertex_map.iter().skip(1) {
                                let mut different = false;
                                for vertex in &ridge_vertices_idx {
                                    if !all_vertices_idx.contains(&row[*vertex]) {
                                        continue 'vmap;
                                    }
                                    if !ridge_vertices_idx.contains(&row[*vertex]) {
                                        different = true;
                                    }
                                }
                                if different {
                                    // We found a coplanar copy of the ridge, thus a disentanglement.
                                    let mut new_ridge = ridge.clone();
        
                                    let mut new_list = ElementList::new();
                                    for i in 0..new_ridge[2].len() {
                                        let mut new = Element::new(Subelements::new(), Superelements::new());
                                        for sub in &ridge[2][i].subs {
                                            new.subs.push(row[*sub])
                                        }
                                        new_list.push(new);
                                    }
                                    new_ridge[2] = new_list;
        
                                    disentangled = Some(new_ridge);
                                    break;
                                }
                            }
                            if let Some(copy) = &disentangled {
                                let mut compound = ridge.clone();
                                compound.append(copy);
                            }
                        }
                        */

                        let mut found = false;

                        for row in &vertex_map {
                            let mut new_ridge = ridge.clone();
                        
                            let mut new_list = ElementList::new();
                            for i in 0..new_ridge[2].len() {
                                let mut new = Element::new(Subelements::new(), Superelements::new());
                                for sub in &ridge[2][i].subs {
                                    new.subs.push(row[*sub])
                                }
                                new_list.push(new);
                            }
                            new_ridge[2] = new_list;

                            new_ridge.element_sort_strong();
                            if let Some((idx, _)) = ridge_orbits.get(&new_ridge) {
                                // writes the orbit index at the ridge index
                                r_i_o_row_row.push(*idx);
                                found = true;
                                break
                            }
                        }

                        if !found {
                            // counts the ridges in the orbit
                            let mut count = 0;
                            let mut set = HashSet::new();

                            for row in &vertex_map {
                                let mut new_ridge = ridge.clone();
                            
                                let mut new_list = ElementList::new();
                                for i in 0..new_ridge[2].len() {
                                    let mut new = Element::new(Subelements::new(), Superelements::new());
                                    for sub in &ridge[2][i].subs {
                                        new.subs.push(row[*sub])
                                    }
                                    new_list.push(new);
                                }
                                new_ridge[2] = new_list;

                                new_ridge.element_sort_strong();
                                if set.get(&new_ridge).is_none() {
                                    set.insert(new_ridge);
                                    count += 1;
                                }
                            }
                            ridge_orbits.insert(ridge, (orbit_idx, count));
                            r_i_o_row_row.push(orbit_idx);
                            ridge_counts.push(count);
                            orbit_idx += 1;
                            
                            if now.elapsed().as_millis() > DELAY {
                                print!("{}{}/{} hp, {} ridges", CL, hp_i, hyperplane_orbits.len(), ridge_orbits.len());
                                std::io::stdout().flush().unwrap();
                                now = Instant::now();
                            }
                        }
                    }
                    r_i_o_row.push(r_i_o_row_row);
                }
                ridge_idx_orbits.push(r_i_o_row);

                print!("{}{}/{} hp, {} ridges", CL, hp_i+1, hyperplane_orbits.len(), ridge_orbits.len());
                std::io::stdout().flush().unwrap();
            }

            // Actually do the faceting
            println!("\n\nCombining...");

            let mut ridge_muls = Vec::new();
            let mut ones = vec![Vec::<(usize, usize)>::new(); ridge_counts.len()];

            for (hp, list) in possible_facets.iter().enumerate() {
                let mut ridge_muls_hp = Vec::new();
                for (f, _) in list.iter().enumerate() {
                    let mut ridge_muls_facet = vec![0; ridge_counts.len()];

                    let f_count = f_counts[hp];
    
                    let ridge_idxs_local = &possible_facets[hp][f].1;
                    for ridge_idx in ridge_idxs_local {
                        let ridge_orbit = ridge_idx_orbits[hp][ridge_idx.0][ridge_idx.1];
                        let ridge_count = ff_counts[hp][ridge_idx.0];
                        let total_ridge_count = ridge_counts[ridge_orbit];
                        let mul = f_count * ridge_count / total_ridge_count;

                        if mul == 1 {
                            ones[ridge_orbit].push((hp, f));
                        }
        
                        ridge_muls_facet[ridge_orbit] = mul;
                    }

                    ridge_muls_hp.push(ridge_muls_facet);
                }
                ridge_muls.push(ridge_muls_hp);
            }

            let mut output_facets = Vec::new();

            let mut facets_queue = VecDeque::<(
                Vec<(usize, usize)>, // list of facets
                usize, // min hyperplane
                Vec<usize> // cached ridge muls
            )>::new();

            for (hp, list) in possible_facets.iter().enumerate() {
                for f in 0..list.len() {
                    facets_queue.push_back((
                        vec![(hp, f)],
                        hp,
                        vec![0; ridge_counts.len()]
                    ));
                }
            }

            while let Some((facets, min_hp, cached_ridge_muls)) = facets_queue.pop_back() {

                if now.elapsed().as_millis() > DELAY {
                    print!("{}", CL);
                    print!("{:.115}", format!("{} facetings, {:?}", output_facets.len(), facets));
                    std::io::stdout().flush().unwrap();
                    now = Instant::now();
                }

                let mut new_ridge_muls = cached_ridge_muls.clone();

                let last_facet = facets.last().unwrap();

                'a: loop {
                    let hp = last_facet.0;
                    let f = last_facet.1;

                    let ridge_idxs_local = &possible_facets[hp][f].1;
                    for ridge_idx in ridge_idxs_local {
                        let ridge_orbit = ridge_idx_orbits[hp][ridge_idx.0][ridge_idx.1];
                        let mul = ridge_muls[hp][f][ridge_orbit];
        
                        new_ridge_muls[ridge_orbit] += mul;
                        if new_ridge_muls[ridge_orbit] > 2 {
                            break 'a;
                        }
                    }
                    break;
                }
                let mut valid = 0; // 0: valid, 1: exotic, 2: incomplete
                for r in &new_ridge_muls {
                    if *r > 2 {
                        valid = 1;
                        break
                    }
                    if *r == 1 {
                        valid = 2;
                    }
                }
                match valid {
                    0 => {
                        // Split compound facets into their components.
                        let mut new_facets = Vec::new();
        
                        for (hp, idx) in &facets {
                            let mut all_components = Vec::<usize>::new();
                            let mut queue = VecDeque::new();
                            queue.push_back(*idx);
                            while let Some(next) = queue.pop_front() {
                                if let Some(components) = compound_facets[*hp].get(&next) {
                                    queue.push_back(components.0);
                                    queue.push_back(components.1);
                                } else {
                                    all_components.push(next);
                                }
                            }
                            for component in all_components {
                                new_facets.push((*hp, component));
                            }
                        }
                        new_facets.sort_unstable();
        
                        output_facets.push(new_facets);

                        if let Some(max_facets) = noble {
                            if facets.len() == max_facets {
                                continue;
                            }
                        }
                        if include_compounds {
                            let mut used_hps = HashSet::new();
                            for facet in facets.iter().skip(1) {
                                used_hps.insert(facet.0);
                            }
                            for (hp, list) in possible_facets.iter().enumerate().skip(min_hp+1) {
                                if !used_hps.contains(&hp) {
                                    for f in 0..list.len() {
                                        let mut new_facets = facets.clone();
                                        new_facets.push((hp, f));
                                        facets_queue.push_back((new_facets, hp, new_ridge_muls.clone()));
                                    }
                                }
                            }
                        }
                    }
                    1 => {}
                    2 => {
                        if let Some(max_facets) = noble {
                            if facets.len() == max_facets {
                                continue;
                            }
                        }
                        let mut used_hps = HashSet::new();
                        for facet in facets.iter().skip(1) {
                            used_hps.insert(facet.0);
                        }
                        for (idx, mul) in new_ridge_muls.iter().enumerate() {
                            if *mul == 1 {
                                for facet in ones[idx]
                                    .iter()
                                    .skip(binary(&ones[idx], min_hp))
                                {
                                    if !used_hps.contains(&facet.0) {
                                        let mut new_facets = facets.clone();
                                        new_facets.push(*facet);
                                        facets_queue.push_back((new_facets, min_hp, new_ridge_muls.clone()));
                                    }
                                }
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            println!("{}{} facetings", CL, output_facets.len());

            output_facets.sort_unstable();

            if !include_compounds {
                println!("\nFiltering mixed compounds...");
                let output_idxs = filter_irc(&output_facets);
                let mut output_new = Vec::new();
                for idx in output_idxs {
                    output_new.push(output_facets[idx].clone());
                }
                output_facets = output_new;
            }

            // Output the faceted polytopes. We will build them from their sets of facet orbits.

            println!("Found {} facetings", output_facets.len());
            println!("\nBuilding...");
            let mut used_facets = HashMap::new(); // used for outputting the facets at the end if `save_facets` is `true`.
            let mut faceting_idx = 0; // We used to use `output.len()` but this doesn't work if you skip outputting the polytopes.

            for facets in output_facets {
                if !save && !save_facets {
                    let mut facets_fmt = String::new();
                    for facet in &facets {
                        facets_fmt.push_str(&format!(" ({},{})", facet.0, facet.1));
                    }
                    println!("Faceting {}:{}", faceting_idx, facets_fmt);

                    faceting_idx += 1;
                    continue
                }

                let mut facet_set = HashSet::new();
                let mut used_facets_current = Vec::new();
                let mut facet_vec = Vec::new();

                if !save {
                    let mut already_found_all = true;
                    for facet in &facets {
                        if used_facets.get(facet).is_none() {
                            already_found_all = false;
                            break
                        }
                    }

                    if already_found_all { 
                        let mut facets_fmt = String::new();
                        for facet in &facets {
                            facets_fmt.push_str(&format!(" ({},{})", facet.0, facet.1));
                        }
                        println!("Faceting {}:{}", faceting_idx, facets_fmt);

                        faceting_idx += 1;
                        continue
                    }
                }

                for facet_orbit in facets.clone() {
                    if save_facets {
                        if used_facets.get(&facet_orbit).is_none() {
                            used_facets_current.push((facet_orbit, facet_set.len()));
                        }
                    }
                    let facet = &possible_facets_global[facet_orbit.0][facet_orbit.1].0;
                    let facet_local = &possible_facets[facet_orbit.0][facet_orbit.1].0;

                    let mut of_this_orbit = HashSet::new();
                    for row in &vertex_map {
                        let mut new_facet = facet.clone();
        
                        let mut new_list = ElementList::new();
                        for i in 0..new_facet[2].len() {
                            let mut new = Element::new(Subelements::new(), Superelements::new());
                            for sub in &new_facet[2][i].subs {
                                new.subs.push(row[*sub])
                            }
                            new_list.push(new);
                        }
                        let mut edges = new_list.clone();
                        for edge in &mut edges {
                            edge.subs.sort();
                        }
                        edges.0.sort_by(|a, b| a.subs.cmp(&b.subs));
                        if let Some(_) = of_this_orbit.get(&edges) {
                            continue;
                        }
                        of_this_orbit.insert(edges);
                        new_facet[2] = new_list;

                        new_facet.element_sort_strong_with_local(facet_local);
                        facet_set.insert(new_facet.clone());
                        facet_vec.push(new_facet); // have to do this so you can predict the facet index
                                                // also it makes the facets sorted by type so that's cool
                    }
                }

                let mut ranks = Ranks::new();
                ranks.push(vec![Element::new(vec![].into(), vec![].into())].into()); // nullitope

                // vertices
                let mut to_new_idx = HashMap::new();
                let mut to_old_idx = Vec::new();
                let mut idx = 0;

                for i in 0..facet_vec.len() {
                    let mut new_list = ElementList::new();
                    for j in 0..facet_vec[i][2].len() {
                        let mut new = Element::new(Subelements::new(), Superelements::new());
                        for sub in facet_vec[i][2][j].subs.clone() {
                            if to_new_idx.get(&sub).is_none() {
                                to_new_idx.insert(sub, idx);
                                to_old_idx.push(sub);
                                idx += 1;
                            }
                            new.subs.push(*to_new_idx.get(&sub).unwrap())
                        }
                        new_list.push(new);
                    }
                    facet_vec[i][2] = new_list;
                }
                let mut new_rank = ElementList::new();
                for _i in 0..idx {
                    new_rank.push(Element::new(vec![0].into(), vec![].into()));
                }
                ranks.push(new_rank);

                for r in 2..rank-1 { // edges and up
                    let mut subs_to_idx = HashMap::new();
                    let mut idx_to_subs = Vec::new();
                    let mut idx = 0;
        
                    for facet in &facet_vec {
                        let els = &facet[r];
                        for el in els {
                            if subs_to_idx.get(&el.subs).is_none() {
                                subs_to_idx.insert(el.subs.clone(), idx);
                                idx_to_subs.push(el.subs.clone());
                                idx += 1;
                            }
                        }
                    }
                    for i in 0..facet_vec.len() {
                        let mut new_list = ElementList::new();
                        for j in 0..facet_vec[i][r+1].len() {
                            let mut new = Element::new(Subelements::new(), Superelements::new());
                            for sub in &facet_vec[i][r+1][j].subs {
                                let sub_subs = &facet_vec[i][r][*sub].subs;
                                new.subs.push(*subs_to_idx.get(sub_subs).unwrap())
                            }
                            new_list.push(new);
                        }
                        facet_vec[i][r+1] = new_list;
                    }
                    let mut new_rank = ElementList::new();
                    for el in idx_to_subs {
                        new_rank.push(Element::new(el, vec![].into()));
                    }
                    ranks.push(new_rank);
                }
        
                let mut new_rank = ElementList::new();
                let mut set = HashSet::new();
        
                for f_i in 0..facet_vec.len() {
                    facet_vec[f_i][rank-1][0].subs.sort();
                    let subs = facet_vec[f_i][rank-1][0].subs.clone();
                    if !set.contains(&subs) {
                        new_rank.push(Element::new(subs.clone(), Superelements::new()));
                        set.insert(subs);
                    }
                }
                let n_r_len = new_rank.len();
                ranks.push(new_rank); // facets
        
                ranks.push(vec![Element::new(Subelements::from_iter(0..n_r_len), Superelements::new())].into()); // body
        
                unsafe {
                    let mut builder = AbstractBuilder::new();
                    for rank in ranks {
                        builder.push_empty();
                        for el in rank {
                            builder.push_subs(el.subs);
                        }
                    }
        
                    if builder.ranks().is_dyadic().is_ok() {
                        let mut abs = builder.build();
                        let mut new_vertices = Vec::new();
                        for i in to_old_idx {
                            new_vertices.push(vertices[i].clone());
                        }

                        let poly = Concrete {
                            vertices: new_vertices,
                            abs: abs.clone(),
                        };

                        let mut fissary_status = "";
                        if mark_fissary {
                            abs.element_sort();
                            
                            if abs.is_compound() {
                                fissary_status = " [C]";
                            } else if poly.is_fissary() {
                                fissary_status = " [F]";
                            }
                        }
                        
                        let mut facets_fmt = String::new();
                        for facet in &facets {
                            facets_fmt.push_str(&format!(" ({},{})", facet.0, facet.1));
                        }

                        if save {
                            let name = format!("faceting {}{}{}{}",
                                if any_single_edge_length {edge_length_idx.to_string() + "."} else {"".to_string()},
                                faceting_idx,
                                if label_facets {" -".to_owned() + &facets_fmt.to_string()} else {"".to_string()},
                                fissary_status
                            );

                            if save_to_file {
                                let mut path = PathBuf::from(&file_path);
                                path.push(format!("{}.off", name));
                                match poly.to_path(&path, Default::default()) {
                                    Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
                                    Ok(_) => (),
                                }
                            } else {
                                output.push((poly.clone(), Some(name)));
                            }
                        }

                        if save_facets {
                            for (orbit, idx) in used_facets_current {
                                used_facets.insert(orbit, poly.facet(idx).unwrap());
                            }
                        }
                        
                        println!("Faceting {}:{}{}", faceting_idx, facets_fmt, fissary_status);

                        faceting_idx += 1;
                    }
                }
            }

            if save_facets {
                let mut used_facets_vec: Vec<(&(usize, usize), &Concrete)> = used_facets.iter().collect();
                used_facets_vec.sort_by(|a,b| a.0.cmp(b.0));

                for i in used_facets_vec {
                    let mut poly = i.1.clone();
                    poly.flatten();
                    if let Some(sphere) = poly.circumsphere() {
                        poly.recenter_with(&sphere.center);
                    } else {
                        poly.recenter();
                    }
                    if save_to_file {
                        let mut path = PathBuf::from(&file_path);
                        path.push(format!("facet ({},{}).off", i.0.0, i.0.1));
                        match poly.to_path(&path, Default::default()) {
                            Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
                            Ok(_) => (),
                        }
                    } else {  
                        output.push((poly, Some(format!("facet ({},{})", i.0.0, i.0.1))));
                    }
                }
            }

            if any_single_edge_length {
                edge_length_idx += 1;
                if edge_length_idx < possible_lengths.len() {
                    continue;
                }
            }

            println!("\nFaceting complete\n");
            return output
        }
    }
}