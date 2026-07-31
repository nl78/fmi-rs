use crate::{
    fmi2::FMU2,
    model_description::fmi2::VariableType,
    sim::{
        SimulationError,
        fmi2::{Trajectories, VariableValue, call},
    },
};

/// Fire-and-forget notification called by [`Recorder`] every time a new row is sampled.
/// Implementations must not block; they run on the simulation thread.
pub trait SampleObserver: Send {
    fn on_sample(&mut self, time: f64, values: &[VariableValue]);
}

pub struct Recorder<'res, 'md> {
    pub simulation_result: &'res mut Trajectories<'md>,
    pub observer: Option<Box<dyn SampleObserver>>,
}

impl<'res, 'md> Recorder<'res, 'md> {
    pub fn new(simulation_result: &'res mut Trajectories<'md>) -> Self {
        Recorder {
            simulation_result,
            observer: None,
        }
    }

    pub fn with_observer(
        simulation_result: &'res mut Trajectories<'md>,
        observer: Box<dyn SampleObserver>,
    ) -> Self {
        Recorder {
            simulation_result,
            observer: Some(observer),
        }
    }

    pub fn sample<I>(&mut self, time: f64, fmu: &FMU2<I>) -> Result<(), SimulationError> {
        self.simulation_result.time.push(time);

        let mut row = vec![];

        for variable in self.simulation_result.variables.iter() {
            let value_references = [variable.valueReference];

            let variable_value = match variable.variableType {
                VariableType::Real { .. } => {
                    let mut values = [0.0];
                    call(fmu.getReal(&value_references, &mut values))?;
                    VariableValue::Real(values[0])
                }
                VariableType::Integer { .. } | VariableType::Enumeration { .. } => {
                    let mut values = [0];
                    call(fmu.getInteger(&value_references, &mut values))?;
                    VariableValue::Integer(values[0])
                }
                VariableType::Boolean { .. } => {
                    let mut values = [0];
                    call(fmu.getBoolean(&value_references, &mut values))?;
                    VariableValue::Boolean(values[0])
                }
                VariableType::String { .. } => {
                    let mut values = [String::new()];
                    call(fmu.getString(&value_references, &mut values))?;
                    VariableValue::String(values[0].clone())
                }
            };

            row.push(variable_value);
        }

        if let Some(observer) = self.observer.as_mut() {
            observer.on_sample(time, &row);
        }

        self.simulation_result.rows.push(row);

        Ok(())
    }
}
