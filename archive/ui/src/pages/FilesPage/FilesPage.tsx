import { GothicPanel } from '../../components/GothicPanel';
import styles from './FilesPage.module.css';

export function FilesPage() {
  return (
    <div className={styles.page}>
      <div>
        <div className={styles.header}>The Archives</div>
        <div className={styles.headerSub}>Repository file explorer</div>
      </div>

      <div className={styles.splitPane}>
        <div className={styles.treePane}>
          <GothicPanel title="File Tree">
            <div className={styles.empty}>
              <div className={styles.emptyIcon}>📂</div>
              Connect file API to browse repository
            </div>
          </GothicPanel>
        </div>
        <div className={styles.viewerPane}>
          <GothicPanel title="Viewer">
            <div className={styles.empty}>
              <div className={styles.emptyIcon}>📜</div>
              Select a file to view its contents
            </div>
          </GothicPanel>
        </div>
      </div>
    </div>
  );
}
