use ndarray::{array, s};
use std::collections::BTreeSet;

use crate::utils::*;

pub trait BaseChannel {
    // Field methods
    fn samp_rate(&self) -> f64;
    fn physical_name(&self) -> &str;
    fn is_fresh_compiled(&self) -> bool;
    fn instr_list(&self) -> &BTreeSet<InstrBook>;
    fn instr_end(&self) -> &Vec<usize>;
    fn instr_val(&self) -> &Vec<Instruction>;
    // Mutable field references
    fn fresh_compiled_(&mut self) -> &mut bool;
    fn instr_list_(&mut self) -> &mut BTreeSet<InstrBook>;
    fn instr_end_(&mut self) -> &mut Vec<usize>;
    fn instr_val_(&mut self) -> &mut Vec<Instruction>;

    // instr_list tracks edits, while (instr_end, instr_list) tracks compilation results
    fn is_compiled(&self) -> bool {
        !self.instr_end().is_empty()
    }
    fn is_edited(&self) -> bool {
        !self.instr_list().is_empty()
    }

    // Pads and merges the contents of instr_list and stores in (instr_end, instr_val)
    fn compile(&mut self, stop_pos: usize) {
        if self.instr_list().len() == 0 {
            // panic!("Attempting to compile channel {} with 0 instructions",
            //         self.physical_name());
            return;
        }
        // Ignore double compiles
        if self.is_fresh_compiled() && *self.instr_end().last().unwrap() == stop_pos {
            return;
        }
        self.clear_compile_cache();
        *self.fresh_compiled_() = true;

        if self.instr_list().last().unwrap().end_pos > stop_pos {
            panic!(
                "Attempting to compile channel {} with stop_pos {} while instructions end at {}",
                self.physical_name(),
                stop_pos,
                self.instr_list().last().unwrap().end_pos
            );
        }

        let mut last_val = 0.;
        let mut last_end = 0;
        let mut instr_val: Vec<Instruction> = Vec::new();
        let mut instr_end: Vec<usize> = Vec::new();

        // Padding, instructions are already sorted
        let samp_rate = self.samp_rate();
        for instr_book in self.instr_list().iter() {
            if last_end != instr_book.start_pos {
                // Add padding instruction
                instr_val.push(Instruction::new_const(last_val));
                instr_end.push(instr_book.start_pos);
            }
            // Add original instruction
            instr_val.push(instr_book.instr.clone());
            instr_end.push(instr_book.end_pos);

            if instr_book.keep_val {
                last_val = match instr_book.instr.instr_type {
                    // Constant instruction: just retrieve its value for future padding
                    InstrType::CONST => *instr_book.instr.args.get("value").unwrap(),
                    // Other instructions: simulate end_pos
                    _ => {
                        let t_end = (instr_book.end_pos as f64) / samp_rate;
                        let mut t_arr = array![t_end];
                        instr_book.instr.eval_inplace(&mut t_arr.view_mut());
                        t_arr[0]
                    }
                };
            } else {
                last_val = 0.;
            }
            last_end = instr_book.end_pos;
        }
        // Pad the last instruction
        if self.instr_list().last().unwrap().end_pos != stop_pos {
            instr_val.push(Instruction::new_const(last_val));
            instr_end.push(stop_pos);
        }

        // Merge instructions, if possible
        for i in 0..instr_end.len() {
            if self.instr_val().is_empty() || instr_val[i] != *self.instr_val().last().unwrap() {
                self.instr_val_().push(instr_val[i].clone());
                self.instr_end_().push(instr_end[i]);
            } else {
                *self.instr_end_().last_mut().unwrap() = instr_end[i];
            }
        }
    }
    // Utility function for compilation: assiming end_instr is in rising order (does not check)
    // Returns the least index such that pos is less than the indexed element
    fn binfind_first_intersect_instr(&self, start_pos: usize) -> usize {
        let mut low: i32 = 0;
        let mut high: i32 = self.instr_end().len() as i32 - 1;
        while low <= high {
            let mid = ((low + high) / 2) as usize;
            if self.instr_end()[mid] < start_pos {
                low = mid as i32 + 1;
            } else if self.instr_end()[mid] > start_pos {
                high = mid as i32 - 1;
            } else {
                return mid as usize;
            }
        }
        low as usize
    }

    fn clear_edit_cache(&mut self) {
        *self.fresh_compiled_() = self.instr_end().len() == 0;
        self.instr_list_().clear();
    }

    fn clear_compile_cache(&mut self) {
        *self.fresh_compiled_() = self.instr_list().len() == 0;
        self.instr_end_().clear();
        self.instr_val_().clear();
    }

    fn compiled_stop_time(&self) -> f64 {
        *self.instr_end().last().unwrap_or(&0) as f64 / self.samp_rate()
    }

    fn edit_stop_time(&self) -> f64 {
        self.instr_list()
            .last()
            .map_or(0., |instr| instr.end_pos as f64 / self.samp_rate())
    }

    // Base method through which to add instruction
    fn add_instr(&mut self, instr: Instruction, t: f64, duration: f64, keep_val: bool) {
        let start_pos = (t * self.samp_rate()) as usize;
        let end_pos = ((t * self.samp_rate()) as usize) + ((duration * self.samp_rate()) as usize);
        let new_instrbook = InstrBook::new(start_pos, end_pos, keep_val, instr);
        // Upon adding an instruction, the channel is not freshly compiled anymore
        *self.fresh_compiled_() = false;

        // Check for overlaps
        let physical_name = self.physical_name();
        if let Some(next) = self.instr_list().range(&new_instrbook..).next() {
            if next.start_pos < new_instrbook.end_pos {
                panic!(
                    "Channel {}\n Instruction {} overlaps with the next instruction {}\n",
                    physical_name, new_instrbook, next
                );
            }
        }
        if let Some(prev) = self.instr_list().range(..&new_instrbook).next_back() {
            if prev.end_pos > new_instrbook.start_pos {
                panic!(
                    "Channel {}\n Instruction {} overlaps with the previous instruction {}",
                    physical_name, new_instrbook, prev
                );
            }
        }

        self.instr_list_().insert(new_instrbook);
    }

    fn constant(&mut self, value: f64, t: f64, duration: f64, keep_val: bool) {
        self.add_instr(Instruction::new_const(value), t, duration, keep_val);
    }

    // Calculates instruction from start_pos to end_pos, with num_samps samples.
    // Assumes that the buffer is written with correctly sampled t-values, and replaces with signal values
    fn fill_signal_nsamps(
        &self,
        start_pos: usize,
        end_pos: usize,
        num_samps: usize,
        buffer: &mut ndarray::ArrayViewMut1<f64>,
    ) {
        assert!(
            self.is_compiled(),
            "Attempting to calculate signal on not-compiled channel {}",
            self.physical_name()
        );
        assert!(
            end_pos > start_pos,
            "Channel {} attempting to calculate signal for invalid interval {}-{}",
            self.physical_name(),
            start_pos,
            end_pos
        );
        assert!(
            end_pos <= (self.compiled_stop_time() * self.samp_rate()) as usize,
            "Attempting to calculate signal interval {}-{} for channel {}, which ends at {}",
            start_pos,
            end_pos,
            self.physical_name(),
            (self.compiled_stop_time() * self.samp_rate()) as usize
        );

        let start_instr_idx: usize = self.binfind_first_intersect_instr(start_pos);
        let end_instr_idx: usize = self.binfind_first_intersect_instr(end_pos);
        // Function for converting position idx (unit of start_pos, end_pos) to buffer offset
        // Linear function: start_pos |-> 0, end_pos |-> num_samps
        let cvt_idx = |pos| {
            ((pos - start_pos) as f64 / (end_pos - start_pos) as f64 * (num_samps as f64)) as usize
        };

        let mut cur_pos: usize = start_pos as usize;
        for i in start_instr_idx..=end_instr_idx {
            let instr_signal_length = std::cmp::min(end_pos, self.instr_end()[i]) - cur_pos;
            let slice =
                &mut buffer.slice_mut(s![cvt_idx(cur_pos)..cvt_idx(cur_pos + instr_signal_length)]);
            self.instr_val()[i].eval_inplace(slice);
            cur_pos += instr_signal_length;
        }
    }
}

pub struct Channel {
    samp_rate: f64,
    fresh_compiled: bool,
    physical_name: String,
    instr_list: BTreeSet<InstrBook>,
    instr_end: Vec<usize>,
    instr_val: Vec<Instruction>,
}

impl BaseChannel for Channel {
    fn samp_rate(&self) -> f64 {
        self.samp_rate
    }
    fn is_fresh_compiled(&self) -> bool {
        self.fresh_compiled
    }
    fn physical_name(&self) -> &str {
        &self.physical_name
    }
    fn instr_list(&self) -> &BTreeSet<InstrBook> {
        &self.instr_list
    }
    fn instr_end(&self) -> &Vec<usize> {
        &self.instr_end
    }
    fn instr_val(&self) -> &Vec<Instruction> {
        &self.instr_val
    }
    fn instr_list_(&mut self) -> &mut BTreeSet<InstrBook> {
        &mut self.instr_list
    }
    fn instr_end_(&mut self) -> &mut Vec<usize> {
        &mut self.instr_end
    }
    fn instr_val_(&mut self) -> &mut Vec<Instruction> {
        &mut self.instr_val
    }
    fn fresh_compiled_(&mut self) -> &mut bool {
        &mut self.fresh_compiled
    }
}

impl Channel {
    pub fn new(physical_name: &str, samp_rate: f64) -> Self {
        Self {
            samp_rate,
            fresh_compiled: true,
            physical_name: physical_name.to_string(),
            instr_list: BTreeSet::new(),
            instr_end: Vec::new(),
            instr_val: Vec::new(),
        }
    }
}
