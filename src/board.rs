use std::collections::{HashMap, HashSet};
use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(EnumIter, Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    SOUTH,
    EAST,
    NORTH,
    WEST,
}

impl Direction {
    fn add(self, point: Coordinate) -> Coordinate {
        match self {
            Direction::NORTH => Coordinate {
                x: point.x + 0,
                y: point.y + -1, // We use the computer graphics convention of (0,0) in the top left
            },
            Direction::SOUTH => Coordinate {
                x: point.x + 0,
                y: point.y + 1,
            },
            Direction::EAST => Coordinate {
                x: point.x + 1,
                y: point.y + 0,
            },
            Direction::WEST => Coordinate {
                x: point.x + -1,
                y: point.y + 0,
            },
        }
    }

    // Returns whether vertical words should be read from top to bottom if played by a player on this side of the board
    fn read_top_to_bottom(self) -> bool {
        matches!(self, Direction::SOUTH) || matches!(self, Direction::WEST)
    }

    fn read_left_to_right(self) -> bool {
        matches!(self, Direction::SOUTH) || matches!(self, Direction::EAST)
    }
}

#[derive(PartialEq, Debug)]
pub struct Board {
    squares: Vec<Vec<Option<Square>>>,
    roots: Vec<Coordinate>,
    orientations: Vec<Direction>, // The side of the board that the player is sitting at, and the direction that their vertical words go in
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        // TODO: is all this internal usize <-> isize conversion worth accepting isize as valid coordinates? Is that only used for simpler traversal algorithms?
        let roots = vec![
            Coordinate {
                x: width as isize / 2 + width as isize % 2 - 1,
                y: 0,
            },
            Coordinate {
                x: width as isize / 2,
                y: height as isize + 1,
            },
        ];

        let mut squares = vec![vec![None; width]]; // Start with an unoccupiable row to house player 1's root
        squares.extend(vec![vec![Some(Square::Empty); width]; height]); // Make the centre of the board empty
        squares.extend(vec![vec![None; width]]); // Add an unoccupiable row to house player 2's root
        squares[roots[0].y as usize][roots[0].x as usize] = Some(Square::Empty); // Create root square
        squares[roots[1].y as usize][roots[1].x as usize] = Some(Square::Empty);

        Board {
            squares,
            roots,
            orientations: vec![Direction::NORTH, Direction::SOUTH],
        }
    }

    pub fn from_string<'a>(
        s: String,
        roots: Vec<Coordinate>,
        orientations: Vec<Direction>,
    ) -> Result<Self, &'a str> {
        if roots.len() != orientations.len() {
            return Err("Every player needs a root and orientation");
        }

        // Transform string into a board
        let mut squares = vec![];
        for line in s.split('\n') {
            let mut squares_in_line: Vec<Option<Square>> = vec![];
            for (i, letter) in line.chars().enumerate() {
                if i % 2 == 1 {
                    if letter != ' ' {
                        return Err("board strings should have spaces to separate each tile");
                    }
                } else if letter == ' ' {
                    squares_in_line.push(None);
                } else if letter == '_' {
                    squares_in_line.push(Some(Square::Empty));
                } else {
                    squares_in_line.push(Some(Square::Occupied(0, letter)));
                }
            }
            squares.push(squares_in_line);
        }

        // Make sure the board is an valid non-jagged grid
        for line in squares.iter().skip(1) {
            if line.len() != squares[0].len() {
                return Err("Unequal line lengths");
            }
        }

        // Make sure letters connected to players' roots are owned by the player
        let r = roots.clone(); // TODO: remove hack
        let mut board = Self {
            roots,
            squares,
            orientations,
        };
        for (player, root) in r.iter().enumerate() {
            if player != 0 {
                // All tiles are already owned by the first player by default
                for square in board.depth_first_search(*root).iter() {
                    if let Ok(Square::Occupied(_, value)) = board.get(*square) {
                        board.set(*square, player, value).expect(
                            "A coordinate returned from a DFS should always be valid and settable",
                        );
                    }
                }
            }
        }

        Ok(board)
    }

    // TODO: generic board constructor that accepts a grid of squares with arbitrary values, as long as:
    //  - the empty squares are fully connected
    //  - there are at least 2 roots
    //  - the roots are at empty squares

    pub fn get(&self, position: Coordinate) -> Result<Square, &str> {
        if position.y < 0 || position.x < 0 {
            return Err("negative coordinates");
        };
        let x = position.x as usize;
        let y = position.y as usize;

        if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                None => Err("Invalid position"),
                Some(square) => Ok(square),
            }
        }
    }

    pub fn set(&mut self, position: Coordinate, player: usize, value: char) -> Result<(), &str> {
        if position.y < 0 || position.x < 0 {
            return Err("negative coordinates");
        };
        let x = position.x as usize;
        let y = position.y as usize;

        if player >= self.roots.len() {
            Err("player does not exist") // TODO: specify the number of players and which player this is
        } else if y >= self.squares.len() {
            Err("y-coordinate is too large for board height") // TODO: specify the coordinate and height
        } else if x >= self.squares[0].len() {
            Err("x-coordinate is too large for board width") // TODO: specify the coordinate and width
        } else {
            match self.squares[y][x] {
                Some(_) => {
                    self.squares[y][x] = Some(Square::Occupied(player, value));
                    Ok(())
                }
                None => Err("Can't set the value of a non-existant square"),
            }
        }
    }

    pub fn get_root(&self, player: usize) -> Result<Coordinate, &str> {
        if player >= self.roots.len() {
            Err("Invalid player")
        } else {
            Ok(self.roots[player])
        }
    }

    pub fn neighbouring_squares(&self, position: Coordinate) -> HashMap<Coordinate, Square> {
        // TODO: does this reinitialise every time even though it's a constant? Or is it compiled into the program?
        let mut neighbours = HashMap::new();
        for delta in Direction::iter() {
            let neighbour_coordinate = delta.add(position);
            match self.get(neighbour_coordinate) {
                Err(_) => {
                    continue; // Skips invalid squares
                }
                Ok(square) => {
                    neighbours.insert(neighbour_coordinate, square);
                }
            }
        }
        neighbours
    }

    // TODO: return iterator or rename since it doesn't matter that this is depth first when we return a HashSet
    fn depth_first_search(&self, position: Coordinate) -> HashSet<Coordinate> {
        let mut set = HashSet::new();

        let player = if let Ok(Square::Occupied(player, _)) = self.get(position) {
            player
        } else {
            return set;
        };
        let mut stack = vec![position]; // TODO: consider more efficient stack type

        while let Some(current) = stack.pop() {
            set.insert(current);
            for neighbour in self.neighbouring_squares(current) {
                // Put the neighbour in the set if it is occupied by the current player
                if let Square::Occupied(neighbours_player, _) = neighbour.1 {
                    if !set.contains(&neighbour.0) && player == neighbours_player {
                        stack.push(neighbour.0);
                    }
                }
            }
        }

        set
    }

    pub fn swap(&mut self, player: usize, positions: [Coordinate; 2]) -> Result<(), &str> {
        if positions[0] == positions[1] {
            return Err("Can't swap a square with itself");
        }

        let mut tiles = ['&'; 2];
        for (i, pos) in positions.iter().enumerate() {
            match self.get(*pos) {
                // TODO: use ? and (possibly) combine function into single match post Polonius
                Err(_) => return Err("Invalid swap position"),
                Ok(square) => match square {
                    Square::Empty => return Err("Must swap between occupied squares"),
                    Square::Occupied(owner, tile) => {
                        if owner != player {
                            return Err("Player must own the squares they swap");
                        }
                        tiles[i] = tile;
                    }
                },
            };
        }

        // TODO: use ? post Polonius
        if self.set(positions[0], player, tiles[1]).is_err() {
            return Err("Can't set");
        }
        self.set(positions[1], player, tiles[0])?;

        Ok(())
    }

    pub fn get_words(&self, position: Coordinate) -> Vec<Vec<Coordinate>> {
        let mut words = Vec::new();
        let mut owner = None;

        for (i, direction) in Direction::iter().enumerate() {
            let mut word = Vec::new();
            let mut location = position;

            'wordbuilder: loop {
                if let Ok(Square::Occupied(player, value)) = self.get(location) {
                    if owner == None {
                        owner = Some(player);
                    }

                    if owner != Some(player) {
                        break 'wordbuilder; // Word ends at other players' letters
                    }

                    word.push(location);
                } else {
                    break 'wordbuilder; // Word ends at the edge of the board or empty squares
                }
                location = direction.add(location);
            }
            if i < 2 {
                words.push(word);
            } else {
                // Combine NORTH/SOUTH and EAST/WEST words
                word.reverse();
                if word.len() > 0 {
                    if words[i - 2].len() > 0 {
                        words[i - 2].splice(0..1, word);
                        // Prepend and remove repeated letter
                    } else {
                        words[i - 2] = word;
                    }
                }
            }
        }

        // Reverse words based on the player's orientation
        if let Some(owner) = owner {
            let orientation = self.orientations[owner];
            if !orientation.read_top_to_bottom() {
                words[0].reverse();
            }
            if !orientation.read_left_to_right() {
                words[1].reverse();
            }
        }

        // 1 letter words don't count
        for i in (0..=1).rev() {
            // TODO: use filter
            if words[i].len() <= 1 {
                words.remove(i);
            }
        }

        words
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new(9, 9)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = self
            .squares
            .iter()
            .map(|row| {
                row.iter()
                    .map(|opt| match opt {
                        Some(sq) => sq.to_string(),
                        None => " ".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", str)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Coordinate {
    pub x: isize,
    pub y: isize,
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Square {
    Empty,
    Occupied(usize, char),
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Square::Empty => write!(f, "_"),
            Square::Occupied(_, tile) => write!(f, "{}", tile),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_default_boards() {
        assert_eq!(
            Board::new(3, 1).to_string(),
            ["  _  ", "_ _ _", "  _  "].join("\n")
        );

        assert_eq!(
            Board::new(3, 2).to_string(),
            ["  _  ", "_ _ _", "_ _ _", "  _  "].join("\n")
        );

        assert_eq!(
            Board::new(2, 1).to_string(),
            ["_  ", "_ _", "  _"].join("\n")
        );

        assert_eq!(
            Board::new(5, 1).to_string(),
            ["    _    ", "_ _ _ _ _", "    _    "].join("\n")
        );

        assert_eq!(
            Board::new(6, 1).to_string(),
            ["    _      ", "_ _ _ _ _ _", "      _    "].join("\n")
        );
    }

    #[test]
    fn from_string() {
        // Checks that our default boards come are the same after being stringified and parsed
        let boards = [Board::default(), Board::new(34, 28)];
        for b in boards {
            assert_eq!(
                Board::from_string(b.to_string(), b.roots.clone(), b.orientations.clone()),
                Ok(b)
            );
        }

        // Checks that various strings are the same when parsed and stringified
        let strings = [
            ["_ _ _", "_   _", "_ _ _"].join("\n"),
            ["_ X _", "_   A", "V _ _"].join("\n"),
            ["_ X _ _", "_ B A _", "V _ _ _", "  _ J _"].join("\n"),
        ];

        // Checks that various complex boards have the correct players assigned to them
        // Donut board
        let top_left = Coordinate { x: 0, y: 0 };
        let top_right = Coordinate { x: 4, y: 0 };
        let bottom_left = Coordinate { x: 0, y: 4 };
        let bottom_right = Coordinate { x: 4, y: 4 };
        let dangling = Coordinate { x: 2, y: 3 };
        let hole = Coordinate { x: 2, y: 2 };
        let donut = if let Ok(t) = Board::from_string(
            [
                "A _ _ _ B",
                "_ _ _ _ _",
                "_ _   _ _",
                "_ _ D _ _",
                "C _ _ _ _",
            ]
            .join("\n"),
            vec![top_left, top_right, bottom_left, bottom_right],
            vec![Direction::NORTH; 4],
        ) {
            t
        } else {
            panic!("Should build");
        };
        assert_eq!(donut.get(top_left), Ok(Square::Occupied(0, 'A')));
        assert_eq!(donut.get(top_right), Ok(Square::Occupied(1, 'B')));
        assert_eq!(donut.get(bottom_left), Ok(Square::Occupied(2, 'C')));
        assert_eq!(donut.get(hole), Err("Invalid position"));
        assert_eq!(donut.get(dangling), Ok(Square::Occupied(0, 'D')));
        assert_eq!(donut.get(Coordinate { x: 1, y: 1 }), Ok(Square::Empty));

        // Complex trees
        let player_1 = [
            Coordinate { x: 2, y: 0 }, // First row
            Coordinate { x: 0, y: 1 }, // Second row
            Coordinate { x: 1, y: 1 },
            Coordinate { x: 2, y: 1 },
            Coordinate { x: 3, y: 1 },
            Coordinate { x: 4, y: 1 },
            Coordinate { x: 0, y: 2 }, // Third row
            Coordinate { x: 0, y: 3 }, // Fourth row
            Coordinate { x: 0, y: 4 }, // Fifth row
            Coordinate { x: 1, y: 4 },
            Coordinate { x: 0, y: 5 }, // Sixth row
        ];
        let player_2 = [
            Coordinate { x: 2, y: 6 }, // Seventh row
            Coordinate { x: 2, y: 5 }, // Sixth row
            Coordinate { x: 3, y: 5 },
            Coordinate { x: 3, y: 4 }, // Fifth row
            Coordinate { x: 2, y: 3 }, // Fourth row
            Coordinate { x: 3, y: 3 },
            Coordinate { x: 4, y: 3 },
        ];
        let complex_tree = if let Ok(t) = Board::from_string(
            [
                "    A    ",
                "A A A A A",
                "A _ _ _ _",
                "A _ B B B",
                "A A _ B _",
                "A _ B B _",
                "    B    ",
            ]
            .join("\n"),
            vec![player_1[0], player_2[0]],
            vec![Direction::NORTH; 2],
        ) {
            t
        } else {
            panic!("Should build");
        };

        for square in player_1 {
            assert_eq!(complex_tree.get(square), Ok(Square::Occupied(0, 'A')));
        }
        for square in player_2 {
            assert_eq!(complex_tree.get(square), Ok(Square::Occupied(1, 'B')));
        }
    }

    #[test]
    fn getset_errors_out_of_bounds() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        assert_eq!(
            b.get(Coordinate { x: 1, y: 0 }),
            Err("x-coordinate is too large for board width")
        );
        assert_eq!(
            b.get(Coordinate { x: 0, y: 3 }),
            Err("y-coordinate is too large for board height")
        );

        assert_eq!(
            b.set(Coordinate { x: 1, y: 0 }, 0, 'a'),
            Err("x-coordinate is too large for board width")
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 3 }, 0, 'a'),
            Err("y-coordinate is too large for board height")
        );
    }

    #[test]
    fn getset_errors_for_dead_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 1, y: 0 }), Err("Invalid position"));
        assert_eq!(b.get(Coordinate { x: 0, y: 2 }), Err("Invalid position"));

        assert_eq!(
            b.set(Coordinate { x: 1, y: 0 }, 0, 'a'),
            Err("Can't set the value of a non-existant square")
        );
        assert_eq!(
            b.set(Coordinate { x: 0, y: 2 }, 0, 'a'),
            Err("Can't set the value of a non-existant square")
        );
    }

    #[test]
    fn getset_handles_empty_squares() {
        let mut b = Board::new(2, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 0, y: 1 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 1, y: 1 }), Ok(Square::Empty));
        assert_eq!(b.get(Coordinate { x: 1, y: 2 }), Ok(Square::Empty));

        assert_eq!(b.set(Coordinate { x: 0, y: 0 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 0, y: 1 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 1, y: 1 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 1, y: 2 }, 0, 'a'), Ok(()));
    }

    #[test]
    fn set_requires_valid_player() {
        let mut b = Board::new(2, 1);
        assert_eq!(b.set(Coordinate { x: 1, y: 2 }, 0, 'a'), Ok(()));
        assert_eq!(b.set(Coordinate { x: 1, y: 2 }, 1, 'a'), Ok(()));
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 2, 'a'),
            Err("player does not exist")
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 3, 'a'),
            Err("player does not exist")
        );
        assert_eq!(
            b.set(Coordinate { x: 1, y: 2 }, 100, 'a'),
            Err("player does not exist")
        );
    }

    #[test]
    fn set_changes_get() {
        let mut b = Board::new(1, 1); // Note, height is 3 from home rows
        assert_eq!(b.get(Coordinate { x: 0, y: 0 }), Ok(Square::Empty));
        assert_eq!(b.set(Coordinate { x: 0, y: 0 }, 0, 'a'), Ok(()));
        assert_eq!(
            b.get(Coordinate { x: 0, y: 0 }),
            Ok(Square::Occupied(0, 'a'))
        );
    }

    #[test]
    fn depth_first_search() {
        let mut b = Board::new(3, 1);

        // Create a connected tree
        let parts = [
            Coordinate { x: 2, y: 1 },
            Coordinate { x: 1, y: 1 },
            Coordinate { x: 1, y: 0 },
            Coordinate { x: 0, y: 1 },
        ];
        let partsSet = HashSet::from(parts);
        for part in parts {
            assert_eq!(b.set(part, 0, 'a'), Ok(()));
        }

        // The tree should be returned no matter where in the tree we start DFS from
        for part in parts {
            assert!(b.depth_first_search(part).is_subset(&partsSet));
            assert!(b.depth_first_search(part).is_superset(&partsSet));
        }

        // Set the remaining unoccupied square on the board to be occupied by another player
        let other = Coordinate { x: 1, y: 2 };
        // WHen unoccupied it should give the empty set, when occupied, just itself
        assert!(b.depth_first_search(other).iter().eq([].iter()));
        assert_eq!(b.set(other, 1, 'a'), Ok(()));
        assert!(b.depth_first_search(other).iter().eq([other].iter()));

        // The result of DFS on the main tree should not have changed
        for part in parts {
            assert!(b.depth_first_search(part).is_subset(&partsSet));
            assert!(b.depth_first_search(part).is_superset(&partsSet));
        }
    }

    #[test]
    fn get_neighbours() {
        // (0,0) (1,0) (2,0)
        // (0,1) (1,1) (2,1)
        // (0,2) (1,2) (2,2)
        // (0,3) (1,3) (2,3)
        // (0,4) (1,4) (2,4)
        let b = Board::new(3, 3);

        assert_eq!(
            // TODO: should we allow you to find neighbours of an invalid square?
            b.neighbouring_squares(Coordinate { x: 0, y: 0 }),
            HashMap::from([
                (Coordinate { x: 0, y: 1 }, Square::Empty),
                (Coordinate { x: 1, y: 0 }, Square::Empty),
            ])
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 0 }),
            HashMap::from([(Coordinate { x: 1, y: 1 }, Square::Empty),])
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 2 }),
            HashMap::from([
                (Coordinate { x: 1, y: 1 }, Square::Empty),
                (Coordinate { x: 0, y: 2 }, Square::Empty),
                (Coordinate { x: 2, y: 2 }, Square::Empty),
                (Coordinate { x: 1, y: 3 }, Square::Empty),
            ])
        );

        assert_eq!(
            b.neighbouring_squares(Coordinate { x: 1, y: 4 }),
            HashMap::from([(Coordinate { x: 1, y: 3 }, Square::Empty),])
        );
    }

    #[test]
    fn swap() {
        let mut b = Board::new(3, 1);
        let c0_1 = Coordinate { x: 0, y: 1 };
        let c1_1 = Coordinate { x: 1, y: 1 };
        let c2_1 = Coordinate { x: 2, y: 1 };
        assert_eq!(b.set(c0_1, 0, 'a'), Ok(()));
        assert_eq!(b.set(c1_1, 0, 'b'), Ok(()));
        assert_eq!(b.set(c2_1, 1, 'c'), Ok(()));

        assert_eq!(b.get(c0_1), Ok(Square::Occupied(0, 'a')));
        assert_eq!(b.get(c1_1), Ok(Square::Occupied(0, 'b')));
        assert_eq!(b.swap(0, [c0_1, c1_1]), Ok(()));
        assert_eq!(b.get(c0_1), Ok(Square::Occupied(0, 'b')));
        assert_eq!(b.get(c1_1), Ok(Square::Occupied(0, 'a')));
        assert_eq!(
            b.swap(0, [c0_1, c0_1]),
            Err("Can't swap a square with itself")
        );
        assert_eq!(
            b.swap(0, [c0_1, c2_1]),
            Err("Player must own the squares they swap")
        );
        assert_eq!(
            b.swap(1, [c0_1, c1_1]),
            Err("Player must own the squares they swap")
        );
    }

    #[test]
    fn get_words() {
        // Should return an empty list of words for all points on an empty board, and for positions off the board
        let empty: Vec<Vec<Coordinate>> = vec![];
        let b = Board::default();
        for x in -2..10 {
            for y in -2..10 {
                assert_eq!(b.get_words(Coordinate { x, y }), empty);
            }
        }

        // Gets two words in the middle of a cross
        let b = if let Ok(board) = Board::from_string(
            [
                "_ _ C _ _",
                "_ _ R _ _",
                "S W O R D",
                "_ _ S _ _",
                "_ _ S _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 0, y: 0 }],
            vec![Direction::SOUTH],
        ) {
            board
        } else {
            panic!("Should build")
        };
        let cross = ([0, 1, 2, 3, 4]).map(|y| Coordinate { x: 2, y }); // TODO: range
        let sword = ([0, 1, 2, 3, 4]).map(|x| Coordinate { x, y: 2 }); // TODO: range
        assert_eq!(b.get_words(Coordinate { x: 2, y: 2 }), vec![cross, sword]);

        let just_cross = ([0, 1, 3, 4]).map(|y| Coordinate { x: 2, y });
        for square in just_cross {
            assert_eq!(b.get_words(square), vec![cross]);
        }

        let just_sword = ([0, 1, 3, 4]).map(|x| Coordinate { x, y: 2 });
        for square in just_sword {
            assert_eq!(b.get_words(square), vec![sword]);
        }

        // Doesn't cross other players
        let mut b = if let Ok(board) = Board::from_string(
            [
                "_ _ C _ _",
                "_ _ R _ _",
                "_ _ O _ _",
                "_ _ S _ _",
                "_ _ S _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 0, y: 0 }, Coordinate { x: 4, y: 4 }],
            vec![Direction::SOUTH, Direction::NORTH],
        ) {
            board
        } else {
            panic!("Should build")
        };
        assert_eq!(
            b.get(Coordinate { x: 2, y: 4 }),
            Ok(Square::Occupied(0, 'S'))
        );
        assert_eq!(b.set(Coordinate { x: 3, y: 4 }, 1, 'O'), Ok(()));
        assert_eq!(b.get_words(Coordinate { x: 2, y: 4 }), vec![cross]); // TODO: check coordinates
    }

    #[test]
    fn get_words_orientations() {
        let corners = vec![
            Coordinate { x: 0, y: 0 }, // Anti-clockwise from the top left
            Coordinate { x: 0, y: 6 },
            Coordinate { x: 6, y: 6 },
            Coordinate { x: 6, y: 0 },
        ];
        let cc = corners.clone();

        let b = if let Ok(board) = Board::from_string(
            [
                "N E Z _ G A N",
                "A _ _ _ _ _ E",
                "G _ _ _ _ _ Z",
                "_ _ _ _ _ _ _",
                "Z _ _ _ _ _ G",
                "E _ _ _ _ _ A",
                "N A G _ Z E N",
            ]
            .join("\n"),
            corners,
            vec![
                Direction::WEST,
                Direction::SOUTH,
                Direction::EAST,
                Direction::NORTH,
            ],
        ) {
            // ZEN NAG
            board
        } else {
            panic!("should work")
        };

        for corner in cc {
            let mut words = b
                .get_words(corner)
                .iter()
                .map(|word| {
                    word.iter()
                        .map(|&square| match b.get(square) {
                            Ok(sq) => match sq {
                                Square::Empty => panic!("shouldn't be empty"),
                                Square::Occupied(_, letter) => letter,
                            },
                            Err(e) => panic!("{}", e),
                        })
                        .collect::<String>()
                })
                .collect::<Vec<String>>();
            words.sort();
            assert_eq!(words, vec!["NAG", "ZEN"]);
        }
    }
}
