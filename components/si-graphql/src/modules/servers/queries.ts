import { checkAuthentication } from "@/modules/auth";
import {
  ServerComponent,
  serverComponentData,
} from "@/datalayer/server-component";
import { GqlRoot, GqlContext, GqlInfo } from "@/app.module";
import {
  GetComponentsInput,
  filterComponents,
} from "@/modules/components/queries";

export async function getServerComponents(
  _obj: GqlRoot,
  args: GetComponentsInput,
  _context: GqlContext,
  info: GqlInfo,
): Promise<ServerComponent[]> {
  let user = await checkAuthentication(info);
  let data: ServerComponent[] = await serverComponentData();
  return filterComponents(data, args, user);
}
