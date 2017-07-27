//! **plot** is the backend that actually renders the plots.
//!
//! Users of **dataplotlib** should not need to access **plot**.

use sdl2;

use sdl2::event::Event;
use sdl2::pixels;
use sdl2::keyboard::Keycode;

use sdl2::gfx::primitives::DrawRenderer;
use sdl2::rect::Point;
use sdl2::render::Renderer;

use std::cmp::min;
use std::time::Duration;
use std::{mem, thread, f64};
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use std::collections::VecDeque;

use plotbuilder::*;


pub struct Plot {
}

pub enum GUITask {
    PlotTask(PlotBuilder2D, Plot),
    Terminate,
}

pub struct PlotGUI<'a> {
    sdl_context: sdl2::Sdl,
    pub windows: HashMap<u32, Window<'a>>,
}

pub struct Window<'a>{
    id: u32,
    renderer: Renderer<'a>,
    xs: Option<Vec<Vec<f64>>>,
    ys: Option<Vec<Vec<f64>>>,
    colors: Option<Vec<[f32; 4]>>,
    plot_bounds: Option<[f64; 4]>,
}

trait Surface {
    fn draw_plots(&mut self, xs: Vec<Vec<f64>>, ys: Vec<Vec<f64>>, colors: Vec<[f32; 4]>, plot_bounds: [f64; 4]);
}

impl<'a> PlotGUI<'a> {
    pub fn run(task_queue: Arc<Mutex<VecDeque<GUITask>>>) -> thread::JoinHandle<()> {
        thread::spawn(|| PlotGUI::event_loop(task_queue))
    }

    fn event_loop(task_queue: Arc<Mutex<VecDeque<GUITask>>>) {
        let sdl_context = sdl2::init().unwrap();
        let mut plot_gui = PlotGUI{sdl_context, windows: HashMap::new()};

        let mut events = plot_gui.sdl_context.event_pump().unwrap();

        'main: loop {
            for event in events.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'main,
                    Event::KeyDown { keycode: Some(Keycode::Escape), window_id, .. } 
                    | Event::Window { win_event: sdl2::event::WindowEvent::Close, window_id, .. } => {
                        if let Some(_) = plot_gui.windows.remove(&window_id) {
                            if plot_gui.windows.is_empty() {
                                break 'main;
                            }
                        }
                    },
                    _ => {}
                }
            }
            loop {
                match task_queue.lock().unwrap().pop_front() {
                    Some(gui_task) => {
                        match gui_task {
                            GUITask::PlotTask(plot_builder, plot) => plot_gui.add_window(plot_builder,
                                                                                         plot),
                            GUITask::Terminate  => if plot_gui.windows.is_empty() {break 'main},
                        }
                    }
                    None => break
                }
            }

            for window in plot_gui.windows.values_mut() {
                window.draw();
            }
            thread::sleep(Duration::from_millis(500));
        }       
    }

    fn add_window(&mut self, plot_builder: PlotBuilder2D, plot: Plot) {
        let sdl_video = self.sdl_context.video().unwrap();
        let window = sdl_video.window("2D plot", 720, 720)
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .unwrap();
        let id = window.id();
        let mut renderer = window.renderer().build().unwrap();
        let mut new_window = Window::new(id, renderer);
        plot.new2d(plot_builder, &mut new_window);
        self.windows.insert(id, new_window);
    }
}

impl<'a> Window<'a> {
    fn new(id: u32, renderer: Renderer<'a>) -> Window {
        Window{id, renderer, xs: None, ys: None, colors: None, plot_bounds: None}
    }

    fn draw(&mut self) {
        let xs = self.xs.as_ref().unwrap();
        let ys = self.ys.as_ref().unwrap();
        let plot_bounds = self.plot_bounds.as_ref().unwrap();
        let colors = self.colors.as_ref().unwrap();

        let bordercol = f32_4_to_color([0.95, 0.95, 0.95, 1.0]);
        let bgcol = f32_4_to_color([1.0, 1.0, 1.0, 1.0]);
        let margin = 0.05;
        let invmargin = 1.0 - margin;

        let x_max = plot_bounds[0];
        let y_max = plot_bounds[1];
        let x_min = plot_bounds[2];
        let y_min = plot_bounds[3];

        let (mut w, mut h) = self.renderer.output_size().unwrap();

        // println!("(w, h) = ({}, {})", w, h);
        let m = min(w, h) as f64;
        let space = m * margin;
        let m = m * invmargin;

        self.renderer.set_draw_color(bgcol);
        self.renderer.clear();
        draw_borders(bordercol, bgcol, space, m, &mut self.renderer);

        let y0 = (m + space) as i16 - point2plot(0.0, y_min, y_max, m, space);
        let xn = m;
        // println!("xn: {}", xn);
        self.renderer.thick_line(space as i16,
                                 y0,
                                 xn as i16,
                                 y0,
                                 2,
                                 pixels::Color::RGBA(0, 0, 0, 255))
                     .unwrap();

        for i in 0..colors.len() {
            let color = colors[i];
            let color_rgba = f32_4_to_color(color);

            let y_inv = (m + space) as i16;
            let yt: Vec<i16> = ys[i].iter().map(|y| y_inv - point2plot(*y, y_min, y_max, m, space)).collect();
            let xt: Vec<i16> = xs[i].iter().map(|x| point2plot(*x, x_min, x_max, m, space)).collect();

            // The number of points
            let len = xs[i].len();
            for i in 0..len - 1 {
                let (xa, ya) = (xt[i + 0], yt[i + 0]);
                let (xb, yb) = (xt[i + 1], yt[i + 1]);
                self.renderer.thick_line(xa, ya, xb, yb, 2, color_rgba).unwrap();
            }
        }
        self.renderer.present();
    }
}

impl<'a> Surface for Window<'a> {
    fn draw_plots(&mut self, xs: Vec<Vec<f64>>, ys: Vec<Vec<f64>>, colors: Vec<[f32; 4]>, plot_bounds: [f64; 4]) {
        self.xs = Some(xs);
        self.ys = Some(ys);
        self.colors = Some(colors);
        self.plot_bounds = Some(plot_bounds);
        self.draw();
    }
}

// pt: a point on a 1 dimensional line segment
// min: the closest point to render on the line segment
// max: the farthest point to render on the line segment
// length: the length of the 1 dimensional window space
// space: the offset from the beginning of the line segment
fn point2plot(pt: f64, min: f64, max: f64, length: f64, space: f64) -> i16 {
    (((pt - min) / (max - min)) * (length - space) + space) as i16
}

fn get_max(user_max: Option<f64>, values: &Vec<f64>) -> f64 {
    if let Some(max) = user_max {
        max
    } else {
        let mut max = *values.first().unwrap();
        for val in values {
            if *val > max {
                max = *val;
            }
        }
        max
    }
}

fn get_min(user_min: Option<f64>, values: &Vec<f64>) -> f64 {
    if let Some(min) = user_min {
        min
    } else {
        let mut min = *values.first().unwrap();
        for val in values {
            if *val < min {
                min = *val;
            }
        }
        min
    }
}

fn f32_4_to_color(col: [f32; 4]) -> pixels::Color {
    pixels::Color::RGBA((col[0] * 255f32) as u8,
                        (col[1] * 255f32) as u8,
                        (col[2] * 255f32) as u8,
                        (col[3] * 255f32) as u8)
}

fn draw_borders(bordercol: pixels::Color, bgcol: pixels::Color, space: f64, m: f64, renderer: &mut Renderer) {
    renderer.set_draw_color(bordercol);
    renderer.clear();

    renderer.rectangle((space - 1.0) as i16,
                   (space - 1.0) as i16,
                   (m - 1.0) as i16,
                   (m - 1.0) as i16,
                   pixels::Color::RGBA(0, 0, 255, 255))
        .unwrap();

    renderer.rectangle((space + 1.0) as i16,
                   (space + 1.0) as i16,
                   (m + 1.0) as i16,
                   (m + 1.0) as i16,
                   bgcol)
        .unwrap();
}

fn set_xy(xy: &Vec<(f64, f64)>, x_vector: &mut Vec<Vec<f64>>, y_vector: &mut Vec<Vec<f64>>) {
    x_vector.push(Vec::new());
    y_vector.push(Vec::new());

    let last_index = x_vector.len() - 1;

    for &(x, y) in xy {
        x_vector[last_index].push(x);
        y_vector[last_index].push(y);
    }
}

fn get_plot_bounds(plot_builder: &PlotBuilder2D, xs: &Vec<Vec<f64>>, ys: &Vec<Vec<f64>>) -> [f64; 4] {

    let mut max_xs: Vec<f64> = Vec::new();
    let mut max_ys: Vec<f64> = Vec::new();
    let mut min_xs: Vec<f64> = Vec::new();
    let mut min_ys: Vec<f64> = Vec::new();

    // Get the plot extremities
    for i in 0..xs.len() {
        max_xs.push(get_max(plot_builder.max_x, &xs[i]));
        max_ys.push(get_max(plot_builder.max_y, &ys[i]));

        min_xs.push(get_min(plot_builder.min_x, &xs[i]));
        min_ys.push(get_min(plot_builder.min_y, &ys[i]));
    }

    let plot_bounds: [f64; 4] = [// Apply the plot extremities to the global extremities
                                 max_xs.iter().cloned().fold(0. / 0., f64::max),
                                 max_ys.iter().cloned().fold(0. / 0., f64::max),
                                 min_xs.iter().cloned().fold(0. / 0., f64::min),
                                 min_ys.iter().cloned().fold(0. / 0., f64::min)];
    println!("bounds: {:?}", plot_bounds);
    plot_bounds
}

impl Plot {
    pub fn new() -> Plot {
        Plot{}
    }
    
    pub fn new2d(&self, plot_builder: PlotBuilder2D, surface: &mut Surface) {

        let mut plot_builder = plot_builder;

        let mut pvs = Vec::new();

        mem::swap(&mut plot_builder.pvs, &mut pvs);

        let mut colors: Vec<[f32; 4]> = Vec::new();
        let mut x_points: Vec<Vec<f64>> = Vec::new();
        let mut y_points: Vec<Vec<f64>> = Vec::new();

        for pv in pvs.drain(..) {
            match pv {
                PlotVals2D::XyColor(ref col, ref xy) => {
                    set_xy(xy, &mut x_points, &mut y_points);
                    colors.push(col.clone());
                }
                _ => (),
            }
        }

        // [MAX_X, MAX_Y, MIN_X, MIN_Y]
        let plot_bounds: [f64; 4] = get_plot_bounds(&plot_builder, &x_points, &y_points);
        surface.draw_plots(x_points, y_points, colors, plot_bounds);
    }
}
