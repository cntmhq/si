import leftHandPath from "./intel/leftHandPath";
import torture from "./intel/torture";
import dockerImage, { CheckQualificationCallbacks } from "./intel/dockerImage";
import k8sDeployment from "./intel/k8sDeployment";
import {
  InferPropertiesReply,
  InferPropertiesRequest,
} from "./controllers/inferProperties";
import { RunCommandCallbacks } from "./controllers/runCommand";

export interface Intel {
  inferProperties?(request: InferPropertiesRequest): InferPropertiesReply;
  checkQualifications?: CheckQualificationCallbacks;
  runCommands?: RunCommandCallbacks;
}

const intel: Record<string, Intel> = {
  leftHandPath,
  torture,
  dockerImage,
  k8sDeployment,
};

export default intel;
