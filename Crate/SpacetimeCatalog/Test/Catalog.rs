//! The catalog read as data.
//!
//! These cases are about what a module would render and what a reader would
//! diff: the six tables, their keys, their types, and which of them may be
//! rewritten. The rule that the probe table cannot hold a private value is a
//! column check here rather than a convention in a comment, because a column is
//! what a reader can select.

use AgenticSpacetimeCatalog::{ColumnTypes, Key, PrimaryKey, TableName, TableNames, Tables};

#[test]
fn the_catalog_describes_exactly_the_six_tables_the_model_needs() {
	assert_eq!(
		TableNames,
		[
			TableName::Capability,
			TableName::Generation,
			TableName::Lease,
			TableName::Probe,
			TableName::Belief,
			TableName::Trace,
		]
	);
	assert_eq!(Tables.iter().map(|Table| Table.Name).collect::<Vec<_>>(), TableNames.to_vec());
	for Name in TableNames {
		assert_eq!(Name.Table().Name, Name);
	}
}

#[test]
fn every_table_declares_one_primary_key_and_it_is_the_column_rows_are_addressed_by() {
	for Name in TableNames {
		let Primary:Vec<&str> = Name
			.Table()
			.Columns
			.iter()
			.filter(|Column| Column.Key == Some(Key::Primary))
			.map(|Column| Column.Name)
			.collect();
		assert_eq!(Primary.len(), 1, "{} declares {} keys", Name.Name(), Primary.len());
		assert_eq!(PrimaryKey(Name), Primary[0]);
	}
	assert_eq!(TableNames.map(PrimaryKey).to_vec(), vec!["id"; 6]);
}

#[test]
fn every_column_uses_a_type_the_catalog_declares() {
	for Name in TableNames {
		for Column in Name.Table().Columns {
			assert!(
				ColumnTypes.contains(&Column.Type),
				"{}.{} is {}",
				Name.Name(),
				Column.Name,
				Column.Type.Name()
			);
			assert!(!Column.Note.is_empty(), "{}.{} has no note", Name.Name(), Column.Name);
		}
	}
}

#[test]
fn only_the_two_current_state_tables_are_mutable_history_is_append_only() {
	assert_eq!(
		TableNames
			.iter()
			.filter(|Name| !Name.Table().AppendOnly)
			.copied()
			.collect::<Vec<_>>(),
		vec![TableName::Lease, TableName::Belief]
	);
}

#[test]
fn the_probe_table_has_no_column_that_could_hold_the_private_value() {
	let Columns:Vec<&str> = Tables
		.iter()
		.find(|Table| Table.Name == TableName::Probe)
		.expect("the catalog describes the probe table")
		.Columns
		.iter()
		.map(|Column| Column.Name)
		.collect();
	for Forbidden in ["value", "truth", "secret", "private", "raw"] {
		assert!(
			!Columns.contains(&Forbidden),
			"probe.{Forbidden} would make the secret selectable"
		);
	}
	assert_eq!(
		Columns,
		vec!["id", "property", "version", "audience", "observed", "at", "generation"]
	);
}

#[test]
fn no_table_in_the_catalog_describes_a_column_named_for_a_secret() {
	for Name in TableNames {
		for Column in Name.Table().Columns {
			for Forbidden in ["secret", "token", "password", "cookie"] {
				assert!(
					!Column.Name.to_lowercase().contains(Forbidden),
					"{}.{} names a credential",
					Name.Name(),
					Column.Name
				);
			}
		}
	}
}

/// An undescribed table is a defect, and the port makes it one the type cannot
/// express rather than one a function throws on.
///
/// The source this was ported from took a string and threw `no such table`;
/// here the table is a [`TableName`], so a rule cannot be handed one the
/// catalog does not describe. What is left of the case is the door a string
/// comes in through: it answers `None`, and every name the type has parses back
/// to itself.
#[test]
fn an_undescribed_table_is_a_defect_not_an_empty_answer() {
	assert_eq!(TableName::Parse("nowhere"), None);
	for Name in TableNames {
		assert_eq!(TableName::Parse(Name.Name()), Some(Name));
	}
}
