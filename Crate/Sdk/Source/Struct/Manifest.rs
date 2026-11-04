//! The eight manifests, embedded.
//!
//! `include_str!` rather than a build script or a serde load at launch: the
//! bytes the crate reports are the bytes that were checked in, and the harness
//! that reads them does not have to start the application. A release descriptor
//! can bind these to a binary later; that is a packaging step, not a source one.
//!
//! The table describes the surface this crate re-exports, which is why it lives
//! here rather than in the binary that prints it: a consumer asking what the
//! workspace ships is asking about the items it just imported. The binary keeps
//! its own path to the same values (`Agentic::All`) by re-exporting this module,
//! so there is one definition and no second answer.
//!
//! The facade itself is not one of the eight. It provides no port, so it has
//! nothing a harness would prepare for and nothing `agentic verify --module`
//! could answer; its own `module.json` declares the `facade` kind and the ports
//! it re-exports, which is a statement about this table rather than an entry in
//! it.

/// One module, as the workspace ships it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest {
	/// The manifest's own name, which is the logical identity across the
	/// migration.
	pub Name:&'static str,
	/// The kind of thing it is: adapter, capability, runtime, workflow,
	/// composition.
	pub Kind:&'static str,
	/// The port it provides.
	pub Port:&'static str,
	/// The retention class the manifest declares.
	pub State:&'static str,
	/// The manifest's bytes, exactly as checked in.
	pub JSON:&'static str,
}

/// Every module this workspace ships, in dependency order.
pub const All:[Manifest; 8] = [
	Manifest {
		Name:"@agentic/runtime",
		Kind:"runtime",
		Port:"agentic.runtime",
		State:"holds-workspace",
		JSON:include_str!("../../../Runtime/module.json"),
	},
	Manifest {
		Name:"@agentic/spacetime-catalog",
		Kind:"capability",
		Port:"agentic.catalog.rules",
		State:"stateless",
		JSON:include_str!("../../../SpacetimeCatalog/module.json"),
	},
	Manifest {
		Name:"@agentic/skill-contract",
		Kind:"capability",
		Port:"agentic.skill.validate",
		State:"stateless",
		JSON:include_str!("../../../SkillContract/module.json"),
	},
	Manifest {
		Name:"@agentic/chat-driver",
		Kind:"capability",
		Port:"agentic.chat.ask",
		State:"holds-transport",
		JSON:include_str!("../../../ChatDriver/module.json"),
	},
	Manifest {
		Name:"@agentic/browser-cdp",
		Kind:"adapter",
		Port:"agentic.browse",
		State:"holds-transport",
		JSON:include_str!("../../../BrowserCDP/module.json"),
	},
	Manifest {
		Name:"@agentic/design-verification",
		Kind:"capability",
		Port:"agentic.verify.design",
		State:"stateless",
		JSON:include_str!("../../../DesignVerification/module.json"),
	},
	Manifest {
		Name:"@agentic/workflow",
		Kind:"workflow",
		Port:"agentic.workflow",
		State:"stateless",
		JSON:include_str!("../../../Workflow/module.json"),
	},
	Manifest {
		Name:"@agentic/composition",
		Kind:"composition",
		Port:"agentic.compose",
		State:"stateless",
		JSON:include_str!("../../../Agentic/module.json"),
	},
];

/// The module with this name, if the workspace ships it.
pub fn Find(Name:&str) -> Option<&'static Manifest> { All.iter().find(|Manifest| Manifest.Name == Name) }
