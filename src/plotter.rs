//! **plotter** provides the `Plotter` object which handles plot creation and lifecycles
//!
//! Each plot runs asynchronously in a background thread. A `Plotter` creates and tracks these background threads.
//!
//! For now, `Plotter::plot2d` is the only supported plotting function. It takes a `PlotBuilder2D` containing all needed information.
//!
//! The `Plotter::join` function allows the thread that owns the `Plotter` to wait until the user has closed all open plot windows before continuing.
use std::thread;
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::VecDeque;

use plotbuilder::PlotBuilder2D;
use plot::Plot;
use plot::GUITask;
use plot::PlotGUI;

use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
static GUI_INITIALISED: AtomicBool = ATOMIC_BOOL_INIT;

pub struct Plotter {
    plot_gui: Option<thread::JoinHandle<()>>,
    task_queue: Arc<Mutex<VecDeque<GUITask>>>,
}

impl Plotter {
    /// `new` creates a new `Plotter` object to manage asynchronous plots
    pub fn new() -> Result<Plotter, String> {
        let task_queue = Arc::new(Mutex::new(VecDeque::new()));
        
        if GUI_INITIALISED.load(Ordering::Relaxed) {
            Err("Only one gui Plotter may be initialised at a time.".to_owned())
        }
        else {
            GUI_INITIALISED.store(true, Ordering::Relaxed);
            Ok(Plotter { 
                plot_gui: Some(PlotGUI::run(task_queue.clone())),
                task_queue,
            })
        }
    }

    /// `plot2d` is currently the only supported plotting function. It takes a `PlotBuilder2D` containing all needed information.
    pub fn plot2d(&mut self, plot_builder: PlotBuilder2D) {
        let plot_task = GUITask::PlotTask(plot_builder, Plot::new());
        self.task_queue.lock().unwrap().push_back(plot_task);
    }

}

impl Drop for Plotter {
    fn drop(&mut self) {
        if let Some(plot_gui) = self.plot_gui.take() {
            // terminate the gui thread if there are no plots open and wait for it
            self.task_queue.lock().unwrap().push_back(GUITask::Terminate);
            plot_gui.join().unwrap();
        }
        GUI_INITIALISED.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use plotbuilder::*;
    use util::*;

    #[test]
    fn plot2d_test() {

        let x = linspace(0, 10, 100);
        let y = (&x).iter().map(|x| x.sin()).collect();
        let xy = zip2(&x, &y);

        let mut pb1 = PlotBuilder2D::new();
        pb1.add_simple_xy(xy);
        let mut plt = Plotter::new();
        plt.plot2d(pb1);
        plt.join();
    }
}
