// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { ChangeSetSpec } from "./ChangeSetSpec";
import type { FuncSpec } from "./FuncSpec";
import type { SchemaSpec } from "./SchemaSpec";
import type { SiPkgKind } from "./SiPkgKind";

export type PkgSpec = {
  kind: SiPkgKind;
  name: string;
  version: string;
  description: string;
  createdAt: string;
  createdBy: string;
  defaultChangeSet: string | null;
  workspacePk: string | null;
  workspaceName: string | null;
  schemas: Array<SchemaSpec>;
  funcs: Array<FuncSpec>;
  changeSets: Array<ChangeSetSpec>;
};
