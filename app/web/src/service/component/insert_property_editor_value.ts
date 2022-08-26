import Bottle from "bottlejs";
import { combineLatest, Observable, take } from "rxjs";
import { switchMap } from "rxjs/operators";
import _ from "lodash";
import { ApiResponse, SDF } from "@/api/sdf";
import { Visibility } from "@/api/sdf/dal/visibility";
import { visibility$ } from "@/observable/visibility";
import { AttributeContext } from "@/api/sdf/dal/attribute";

export interface InsertPropertyEditorValueArgs {
  parentAttributeValueId: number;
  attributeContext: AttributeContext;
  value?: unknown;
  key?: string;
}

export interface InsertPropertyEditorValueRequest
  extends InsertPropertyEditorValueArgs,
    Visibility {}

export interface InsertFromEditFieldResponse {
  success: boolean;
}

export function insertFromEditField(
  args: InsertPropertyEditorValueArgs,
): Observable<ApiResponse<InsertFromEditFieldResponse>> {
  const bottle = Bottle.pop("default");
  const sdf: SDF = bottle.container.SDF;
  return combineLatest([visibility$]).pipe(
    take(1),
    switchMap(([visibility]) => {
      const request: InsertPropertyEditorValueRequest = {
        ...args,
        ...visibility,
      };
      return sdf.post<ApiResponse<InsertFromEditFieldResponse>>(
        "component/insert_property_editor_value",
        request,
      );
    }),
  );
}
