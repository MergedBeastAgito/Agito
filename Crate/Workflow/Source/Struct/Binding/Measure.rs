//! Design verification's `Browse`, over the browser substrate.
//!
//! The capability's port has one verb and no session, so this binding opens the
//! configured page for each measurement and releases it afterwards. That is not
//! an oversight: the manifest says `stateless`, and a binding that kept a page
//! alive between measurements would turn a stateless capability into a
//! `holds-transport` one behind its own manifest.
//!
//! The expression this binding sends is the one place a page convention lives,
//! and it is deliberately in the workflow rather than in the verification
//! package: the package must not know what an element is, and a workflow is
//! exactly the layer that decides how a particular site is read.

use std::sync::Arc;

use AgenticBrowserCDP::{Browse as BrowserBrowse, Error as BrowserError, OpenOptions};
use AgenticDesignVerification::{Browse, Failure};
use AgenticRuntime::{Call, FutureBox};
use serde_json::Value;

use crate::Fn::Measure::MeasurementExpression;

/// The verification capability's transport, served by the browser substrate.
pub struct Measure {
	Browse:Arc<dyn BrowserBrowse>,
	URL:String,
}

/// Bind design verification's `Browse` to one page of one browser.
pub fn Create(Browse:Arc<dyn BrowserBrowse>, URL:impl Into<String>) -> Measure { Measure { Browse, URL:URL.into() } }

/// A browser failure, said in the verification port's vocabulary.
fn Classify(Why:BrowserError) -> Failure {
	Failure {
		Message:format!("{}: {}", Why.Code().Name(), Why.Message()),
		Detail:Why.Detail().to_string(),
	}
}

impl Browse for Measure {
	fn Measure<'a>(&'a self, Selectors:&'a [String], Call:Call) -> FutureBox<'a, Result<Vec<Value>, Failure>> {
		Box::pin(async move {
			let Target = self
				.Browse
				.Open(&self.URL, OpenOptions::Launch(Call.clone()))
				.await
				.map_err(Classify)?;
			let Expression = MeasurementExpression(Selectors);
			let Read = self.Browse.Evaluate(&Target, &Expression, Call).await;
			// Released on every path: the capability is stateless, so the page
			// must not outlive the measurement that asked for it.
			self.Browse.Dispose(Target).await;
			match Read {
				Ok(Value::Array(Entries)) => Ok(Entries),
				Ok(Other) => {
					Err(Failure::Of(format!(
						"the page answered {Other} instead of a list of measurements"
					)))
				},
				Err(Why) => Err(Classify(Why)),
			}
		})
	}
}
