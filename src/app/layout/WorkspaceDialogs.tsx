import {SearchDialog} from '../../features/library/SearchDialog';
import { useUiStore } from '../ui-store';
import { Modal } from '../../shared/ui/Modal';
import { CaptureControls } from '../../features/capture/CaptureControls';

export function WorkspaceDialogs() {
  const { overlay, closeOverlay } = useUiStore();
  if (overlay === 'capture') return <Modal key="capture" title="新建截图" onClose={closeOverlay}>
    <div className="p-5">
      <CaptureControls onCaptured={() => useUiStore.getState().navigate('screenshots')} />
    </div>
  </Modal>;
  if (overlay === 'search') return <SearchDialog/>;
  return null;
}
