import {
  assertEquals,
  assertExists,
  assertInstanceOf,
  assertThrows,
} from "@std/assert";
import { generateSiSpecForService } from "../src/commands/generateSiSpecs.ts";
import { ExpandedPropSpec } from "../src/spec/props.ts";
import { loadCfDatabase } from "../src/cfDb.ts";
import { dirname, fromFileUrl, join } from "@std/path";

// const SET_BOOLEAN =
//   "577a7deea25cfad0d4b2dd1e1f3d96b86b8b1578605137b8c4128d644c86964b";
// const SET_INTEGER =
//   "7d384b237852f20b8dec2fbd2e644ffc6bde901d7dc937bd77f50a0d57e642a9";
// const SET_MAP =
//   "dea5084fbf6e7fe8328ac725852b96f4b5869b14d0fe9dd63a285fa876772496";
// const SET_OBJECT =
//   "cb9bf94739799f3a8b84bcb88495f93b27b47c31a341f8005a60ca39308909fd";

const TEST_FILES = join(dirname(fromFileUrl(import.meta.url)), "test-files");
await loadCfDatabase({ path: TEST_FILES });

Deno.test(function generateServiceByName() {
  // Throws if the service does not exist
  assertThrows(() => generateSiSpecForService("poop"));
  // Returns the result if it does
  const goodResult = generateSiSpecForService("AWS::EC2::VPC");
  assertEquals(goodResult.name, "AWS::EC2::VPC");

  assertInstanceOf(goodResult.schemas, Array);
  assertEquals(goodResult.schemas.length, 1);
  assertInstanceOf(goodResult.schemas[0].variants, Array);
  assertEquals(goodResult.schemas[0].variants.length, 1);

  const domain = goodResult.schemas[0].variants[0].domain;
  assertInstanceOf(domain, Object);
  assertInstanceOf(domain.metadata.propPath, Array);
  assertEquals(domain.metadata.propPath, ["root", "domain"]);
  validateProps(domain);
});

function validateProps(prop: ExpandedPropSpec) {
  switch (prop.kind) {
    case "boolean":
      assertEquals(prop.data?.widgetKind, "Checkbox");
      break;
    case "number":
    case "string":
      assertEquals(prop.data?.widgetKind, "Text");
      break;
    case "json":
      break;
    case "array":
      assertEquals(prop.data?.widgetKind, "Array");
      assertExists(prop.typeProp);
      validateProps(prop.typeProp);
      break;
    case "map":
      assertEquals(prop.data?.widgetKind, "Map");
      assertExists(prop.typeProp);
      validateProps(prop.typeProp);
      break;
    case "object":
      assertEquals(prop.data?.widgetKind, "Header");
      assertExists(prop.entries);
      Object.values(prop.entries).forEach((entry) => {
        validateProps(entry);
      });
      break;
    default:
      break;
  }
}
