//! The ten manifests, embedded.
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
//! The facade itself is not one of the ten. It provides no port, so it has
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
	/// The verbs it declares under `provides.verbs`, in its own order.
	///
	/// A verb is an extra thing the module can be asked to do beyond
	/// answering its port - the chat driver's `agentic.chat.note` is the
	/// one the workspace ships - and most modules declare none, so an empty
	/// slice is the ordinary case rather than a missing value.
	pub Verbs:&'static [&'static str],
	/// The retention class the manifest declares.
	pub State:&'static str,
	/// The manifest's bytes, exactly as checked in.
	pub JSON:&'static str,
}

/// Every module this workspace ships, in dependency order.
pub const All:[Manifest; 10] = [
	Manifest {
		Name:"@agentic/runtime",
		Kind:"runtime",
		Port:"agentic.runtime",
		Verbs:&[],
		State:"holds-workspace",
		JSON:include_str!("../../../Runtime/module.json"),
	},
	// The port shape a third-party substrate implements. Kind `capability` and
	// not `adapter`: the boundary rule lets an adapter name a capability's port
	// shapes, which is the edge a third-party adapter draws when it depends on
	// this crate, while an adapter naming another adapter is the edge the rule
	// forbids. Its manifest names no errors, because a code belongs to the
	// adapter that reports it rather than to the shape it implements.
	Manifest {
		Name:"@agentic/adapter",
		Kind:"capability",
		Port:"agentic.adapter",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../Adapter/module.json"),
	},
	// The template corpus as data. It depends on the runtime and nothing else:
	// the round trip leaves through a `Poster` this crate declares and never
	// implements, so its manifest says `network: false` and means it.
	Manifest {
		Name:"@agentic/lane",
		Kind:"capability",
		Port:"agentic.lane.decide",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../Lane/module.json"),
	},
	Manifest {
		Name:"@agentic/spacetime-catalog",
		Kind:"capability",
		Port:"agentic.catalog.rules",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../SpacetimeCatalog/module.json"),
	},
	Manifest {
		Name:"@agentic/skill-contract",
		Kind:"capability",
		Port:"agentic.skill.validate",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../SkillContract/module.json"),
	},
	// The one module that declares a verb. `agentic.chat.note` is not the
	// port: a caller asks `agentic.chat.ask` and gets an answer, while `note`
	// is the call that prints a research note built from that same answer.
	// It carries its own (empty) error list, so the verb is a statement a
	// harness can read rather than a name nobody lists.
	Manifest {
		Name:"@agentic/chat-driver",
		Kind:"capability",
		Port:"agentic.chat.ask",
		Verbs:&["agentic.chat.note"],
		State:"holds-transport",
		JSON:include_str!("../../../ChatDriver/module.json"),
	},
	Manifest {
		Name:"@agentic/browser-cdp",
		Kind:"adapter",
		Port:"agentic.browse",
		Verbs:&[],
		State:"holds-transport",
		JSON:include_str!("../../../BrowserCDP/module.json"),
	},
	Manifest {
		Name:"@agentic/design-verification",
		Kind:"capability",
		Port:"agentic.verify.design",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../DesignVerification/module.json"),
	},
	Manifest {
		Name:"@agentic/workflow",
		Kind:"workflow",
		Port:"agentic.workflow",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../Workflow/module.json"),
	},
	Manifest {
		Name:"@agentic/composition",
		Kind:"composition",
		Port:"agentic.compose",
		Verbs:&[],
		State:"stateless",
		JSON:include_str!("../../../Agentic/module.json"),
	},
];

/// The module with this name, if the workspace ships it.
pub fn Find(Name:&str) -> Option<&'static Manifest> { All.iter().find(|Manifest| Manifest.Name == Name) }
