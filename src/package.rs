// Copyright (c) 2017, Maarten de Vries
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notice, this
//   list of conditions and the following disclaimer.
//
// * Redistributions in binary form must reproduce the above copyright notice,
//   this list of conditions and the following disclaimer in the documentation
//   and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::collections::BTreeMap as Map;

use version::Version;

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Constraint {
	Equal,
	Greater,
	GreaterEqual,
	Less,
	LessEqual,
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct VersionedTarget<'a> {
	pub name: &'a str,
	pub version: Version<'a>,
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct VersionConstraint<'a> {
	pub version:    Version<'a>,
	pub constraint: Constraint,
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct Package<'a> {
	pub pkgname:       &'a str,
	pub epoch:         i32,
	pub pkgver:        &'a str,
	pub pkgrel:        Option<&'a str>,

	pub url:           Option<&'a str>,
	pub description:   Option<&'a str>,
	pub licenses:      Vec<&'a str>,

	pub groups:        Vec<&'a str>,
	pub backup:        Vec<&'a str>,

	pub provides:      Map<&'a str, Option<Version<'a>>>,
	pub conflicts:     Map<&'a str, Option<VersionConstraint<'a>>>,
	pub replaces:      Map<&'a str, Option<VersionConstraint<'a>>>,

	pub depends:       Map<&'a str, Option<VersionConstraint<'a>>>,
	pub opt_depends:   Map<&'a str, Option<VersionConstraint<'a>>>,
	pub make_depends:  Map<&'a str, Option<VersionConstraint<'a>>>,
	pub check_depends: Map<&'a str, Option<VersionConstraint<'a>>>,
}

impl<'a> Package<'a> {
	pub fn version(&self) -> Version<'a> {
		Version{epoch: self.epoch, pkgver: self.pkgver, pkgrel: self.pkgrel}
	}
}

/// A partial package with some information possibly missing.
#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub struct PartialPackage<'a> {
	pub pkgname:       Option<&'a str>,
	pub epoch:         Option<i32>,
	pub pkgver:        Option<&'a str>,
	pub pkgrel:        Option<&'a str>,

	pub url:           Option<&'a str>,
	pub description:   Option<&'a str>,
	pub licenses:      Option<Vec<&'a str>>,

	pub groups:        Option<Vec<&'a str>>,
	pub backup:        Option<Vec<&'a str>>,

	pub provides:      Option<Map<&'a str, Option<Version<'a>>>>,
	pub conflicts:     Option<Map<&'a str, Option<VersionConstraint<'a>>>>,
	pub replaces:      Option<Map<&'a str, Option<VersionConstraint<'a>>>>,

	pub depends:       Option<Map<&'a str, Option<VersionConstraint<'a>>>>,
	pub opt_depends:   Option<Map<&'a str, Option<VersionConstraint<'a>>>>,
	pub make_depends:  Option<Map<&'a str, Option<VersionConstraint<'a>>>>,
	pub check_depends: Option<Map<&'a str, Option<VersionConstraint<'a>>>>,
}

impl<'a> PartialPackage<'a> {
	/// Try to create a package from the partial package.
	pub fn into_package(self) -> Result<Package<'a>, String> {
		Ok(Package {
			pkgname:    self.pkgname.ok_or_else(|| String::from("missing pkgname"))?,
			epoch:      self.epoch.unwrap_or(0),
			pkgver:     self.pkgver.ok_or_else(|| String::from("missing pkgver"))?,
			pkgrel:     self.pkgrel,

			url:           self.url,
			description:   self.description,
			licenses:      self.licenses.unwrap_or_default(),

			groups:        self.groups.unwrap_or_default(),
			backup:        self.backup.unwrap_or_default(),

			provides:      self.provides.unwrap_or_default(),
			conflicts:     self.conflicts.unwrap_or_default(),
			replaces:      self.replaces.unwrap_or_default(),

			depends:       self.depends.unwrap_or_default(),
			opt_depends:   self.opt_depends.unwrap_or_default(),
			make_depends:  self.make_depends.unwrap_or_default(),
			check_depends: self.check_depends.unwrap_or_default(),
		})
	}

	/// Add information from a pkgbase (for split packages).
	pub fn add_base(&mut self, base: &PartialPackage<'a>) {
		if self.url.is_none()         { self.url         = base.url              }
		if self.description.is_none() { self.description = base.description      }
		if self.licenses.is_none()    { self.licenses    = base.licenses.clone() }

		if self.groups.is_none()      { self.groups      = base.groups.clone()   }
		if self.backup.is_none()      { self.backup      = base.backup.clone()   }

		if self.provides.is_none()    { self.provides    = base.provides.clone()    }
		if self.conflicts.is_none()   { self.conflicts   = base.conflicts.clone()   }
		if self.replaces.is_none()    { self.replaces    = base.replaces.clone()    }

		if self.depends.is_none()     { self.depends     = base.depends.clone()     }
		if self.opt_depends.is_none() { self.opt_depends = base.opt_depends.clone() }
	}
}
