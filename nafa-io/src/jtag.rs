use std::{
    collections::VecDeque,
    fmt::Display,
    ops::{Index, IndexMut},
    sync::LazyLock,
};

use strum::VariantArray;

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::VariantArray)]
#[repr(u8)]
pub enum State {
    TestLogicReset,
    RunTestIdle,
    SelectDR,
    CaptureDR,
    ShiftDR,
    Exit1DR,
    PauseDR,
    Exit2DR,
    UpdateDR,
    SelectIR,
    CaptureIR,
    ShiftIR,
    Exit1IR,
    PauseIR,
    Exit2IR,
    UpdateIR,
}

pub struct Edges([State; 2]);
impl State {
    pub const fn edges(self) -> Edges {
        match self {
            State::TestLogicReset => Edges([State::RunTestIdle, State::TestLogicReset]),
            State::RunTestIdle => Edges([State::RunTestIdle, State::SelectDR]),
            State::SelectDR => Edges([State::CaptureDR, State::SelectIR]),
            State::CaptureDR => Edges([State::ShiftDR, State::Exit1DR]),
            State::ShiftDR => Edges([State::ShiftDR, State::Exit1DR]),
            State::Exit1DR => Edges([State::PauseDR, State::UpdateDR]),
            State::PauseDR => Edges([State::PauseDR, State::Exit2DR]),
            State::Exit2DR => Edges([State::ShiftDR, State::UpdateDR]),
            State::UpdateDR => Edges([State::RunTestIdle, State::SelectDR]),
            State::SelectIR => Edges([State::CaptureIR, State::TestLogicReset]),
            State::CaptureIR => Edges([State::ShiftIR, State::Exit1IR]),
            State::ShiftIR => Edges([State::ShiftIR, State::Exit1IR]),
            State::Exit1IR => Edges([State::PauseIR, State::UpdateIR]),
            State::PauseIR => Edges([State::PauseIR, State::Exit2IR]),
            State::Exit2IR => Edges([State::ShiftIR, State::UpdateIR]),
            State::UpdateIR => Edges([State::RunTestIdle, State::SelectDR]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Path {
    path: u8,
    pub len: u8,
}

impl Path {
    /// Transition to TLR, from any state
    pub const RESET: Self = Self { path: 0xff, len: 5 };
    /// Transition to RTI, from any state
    pub const IDLE: Self = Self { path: 0x3e, len: 6 };

    pub const fn as_clocked(self) -> u8 {
        self.path.reverse_bits() >> (8 - self.len)
    }
}

pub struct Graph<T>(pub [T; State::VARIANTS.len()]);
pub const GRAPH: Graph<Edges> = Graph([
    State::TestLogicReset.edges(),
    State::RunTestIdle.edges(),
    State::SelectDR.edges(),
    State::CaptureDR.edges(),
    State::ShiftDR.edges(),
    State::Exit1DR.edges(),
    State::PauseDR.edges(),
    State::Exit2DR.edges(),
    State::UpdateDR.edges(),
    State::SelectIR.edges(),
    State::CaptureIR.edges(),
    State::ShiftIR.edges(),
    State::Exit1IR.edges(),
    State::PauseIR.edges(),
    State::Exit2IR.edges(),
    State::UpdateIR.edges(),
]);

pub static PATHS: LazyLock<Graph<Graph<Path>>> = LazyLock::new(|| {
    let mut ret = Graph(
        [const { Graph([Path { path: 0, len: 0 }; State::VARIANTS.len()]) }; State::VARIANTS.len()],
    );
    for start in State::VARIANTS {
        for end in State::VARIANTS {
            let path = get_path(*start, *end);
            ret[*start][*end] = path;
        }
    }
    ret
});

impl<T> Index<State> for Graph<T> {
    type Output = T;
    fn index(&self, index: State) -> &Self::Output {
        self.0.index(index as u8 as usize)
    }
}

impl<T> IndexMut<State> for Graph<T> {
    fn index_mut(&mut self, index: State) -> &mut Self::Output {
        self.0.index_mut(index as u8 as usize)
    }
}

impl Index<bool> for Edges {
    type Output = State;
    fn index(&self, index: bool) -> &Self::Output {
        self.0.index(index as u8 as usize)
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write as _;
        for bit in *self {
            f.write_char(if bit { '1' } else { '0' })?;
        }
        Ok(())
    }
}

fn get_path(start: State, end: State) -> Path {
    let mut possible_paths = VecDeque::from([
        (Path { path: 0, len: 1 }, GRAPH[start][false]),
        (Path { path: 1, len: 1 }, GRAPH[start][true]),
    ]);

    loop {
        let (cur_path, cur_end) = possible_paths.pop_front().unwrap();
        if cur_end == end {
            return cur_path;
        }

        let p0 = Path {
            path: cur_path.path << 1,
            len: cur_path.len + 1,
        };
        let p1 = Path {
            path: (cur_path.path << 1) | 1,
            len: cur_path.len + 1,
        };
        possible_paths.push_back((p0, GRAPH[cur_end][false]));
        possible_paths.push_back((p1, GRAPH[cur_end][true]));
    }
}

impl IntoIterator for Path {
    type Item = bool;
    type IntoIter = PathIter;
    fn into_iter(self) -> Self::IntoIter {
        PathIter(self, 0)
    }
}

pub struct PathIter(Path, u8);
impl Iterator for PathIter {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.1;
        self.1 = self.1.saturating_add(1);
        if idx < self.0.len {
            Some(self.0.path >> (self.0.len - idx - 1) & 1 == 1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use strum::VariantArray;

    use super::*;

    fn follow_path(start: State, path: Path) -> State {
        let mut cur = start;
        for dir in path {
            cur = GRAPH[cur][dir];
        }
        cur
    }

    #[test]
    fn test_path() {
        for start in State::VARIANTS {
            for end in State::VARIANTS {
                let path = PATHS[*start][*end];
                let result = follow_path(*start, path);
                assert!(
                    *end == result,
                    "
goal:   {start:?} -> {end:?}
path:   {path}
result: {result:?}",
                );
            }
        }
    }
}
