use esp_backtrace as _;

use esp_println::println;

use embedded_graphics::{
    draw_target::Cropped,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
};

const SNAKE_SIZE: usize = 7;

pub enum DirectionChange {
    Left,
    Right,
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct Snake {
    body: [Point; SNAKE_SIZE],
    direction: Direction,
}

// Desc: Snake game logic
pub struct Game {
    snake: Snake,
    score: u32,
    game_over: bool,
}

impl Game {
    pub fn new() -> Self {
        let snake = Snake {
            body: [Point::new(0, 0); SNAKE_SIZE],
            direction: Direction::Right,
        };

        Self {
            snake,
            score: 0,
            game_over: false,
        }
    }

    fn area<D>(area: &mut D) -> Cropped<D>
    where
        D: DrawTarget + Dimensions,
        D::Color: RgbColor,
    {
        let top_left = Point::new(2, 107);
        let size = Size::new(130, 130);

        area.cropped(&Rectangle::new(top_left, size))
    }

    pub fn init<D>(&mut self, display: &mut D)
    where
        D: DrawTarget + Dimensions,
        D::Color: RgbColor,
    {
        self.snake.direction = Direction::Right;
        self.score = 0;
        self.game_over = false;

        let mut area = Self::area(display);

        match area.clear(RgbColor::BLACK) {
            Ok(_) => {}
            Err(_) => {
                println!("Error clearing area");
            }
        }

        let mut points = self.snake.body;

        // Draw snake
        for i in 0..SNAKE_SIZE {
            points[i].x = i as i32;

            self.draw_square(&mut area, points[i]);
        }
    }

    /// draws squares on the whole area
    fn draw_square<D>(&mut self, area: &mut Cropped<D>, point: Point)
    where
        D: DrawTarget + Dimensions,
        D::Color: RgbColor,
    {
        let size = Size::new(10, 10);
        let point = Point {
            x: point.x * 10,
            y: point.y * 10,
        };

        match Rectangle::new(point, size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(RgbColor::WHITE)
                    .stroke_color(RgbColor::BLACK)
                    .stroke_width(1)
                    .build(),
            )
            .draw(area)
        {
            Ok(_) => {}
            Err(_) => {
                println!("Error drawing square");
            }
        }
    }

    fn clear_square<D>(&mut self, area: &mut Cropped<D>, point: Point)
    where
        D: DrawTarget + Dimensions,
        D::Color: RgbColor,
    {
        let size = Size::new(10, 10);
        let point = Point {
            x: point.x * 10,
            y: point.y * 10,
        };

        match Rectangle::new(point, size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(RgbColor::BLACK)
                    .build(),
            )
            .draw(area)
        {
            Ok(_) => {}
            Err(_) => {
                println!("Error drawing square");
            }
        }
    }

    pub fn move_snake<D>(&mut self, display: &mut D)
    where
        D: DrawTarget + Dimensions,
        D::Color: RgbColor,
    {
        let mut new_head = self.snake.body[SNAKE_SIZE - 1].clone();

        let mut area = Self::area(display);

        match self.snake.direction {
            Direction::Up => {
                new_head.y -= 1;
            }
            Direction::Down => {
                new_head.y += 1;
            }
            Direction::Left => {
                new_head.x -= 1;
            }
            Direction::Right => {
                new_head.x += 1;
            }
        }

        if new_head.x > 12 {
            new_head.x = 0;
        } else if new_head.x < 0 {
            new_head.x = 12;
        }

        if new_head.y > 12 {
            new_head.y = 0;
        } else if new_head.y < 0 {
            new_head.y = 12;
        }

        self.clear_square(&mut area, self.snake.body[0]);

        for i in 0..SNAKE_SIZE - 1 {
            // println!("i: {} {:?} < {:?}", i, self.snake.body[i], self.snake.body[i + 1]);
            self.snake.body[i] = self.snake.body[i + 1];
        }

        self.snake.body[SNAKE_SIZE - 1] = new_head;

        self.draw_square(&mut area, new_head);
    }

    pub fn change_direction(&mut self, direction: DirectionChange) {
        match self.snake.direction {
            Direction::Up => match direction {
                DirectionChange::Left => {
                    self.snake.direction = Direction::Left;
                }
                DirectionChange::Right => {
                    self.snake.direction = Direction::Right;
                }
            },
            Direction::Down => match direction {
                DirectionChange::Left => {
                    self.snake.direction = Direction::Right;
                }
                DirectionChange::Right => {
                    self.snake.direction = Direction::Left;
                }
            },
            Direction::Left => match direction {
                DirectionChange::Left => {
                    self.snake.direction = Direction::Down;
                }
                DirectionChange::Right => {
                    self.snake.direction = Direction::Up;
                }
            },
            Direction::Right => match direction {
                DirectionChange::Left => {
                    self.snake.direction = Direction::Up;
                }
                DirectionChange::Right => {
                    self.snake.direction = Direction::Down;
                }
            },
        }
    }
}
