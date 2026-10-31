from pathlib import Path
import difflib,hashlib,json
r=Path(__file__).parent
before=(r/'entry-lib.rs').read_text()
old='''        let backup = rusqlite::backup::Backup::new(source, &mut destination)
            .map_err(Self::projection_backup_error)?;
        loop {
            match backup.step(256).map_err(Self::projection_backup_error)? {
                rusqlite::backup::StepResult::Done => return Ok(()),
                rusqlite::backup::StepResult::More => {}
                rusqlite::backup::StepResult::Busy | rusqlite::backup::StepResult::Locked => {
                    return Err(Error::from_reason(
                        "PROJECTION_REBUILD_BUSY: projection publication could not acquire SQLite locks",
                    ));
                }
                _ => {
                    return Err(Error::from_reason(
                        "PROJECTION_REBUILD_FAILED: SQLite backup returned an unsupported step state",
                    ));
                }
            }
        }
'''
new='''        {
            let backup = rusqlite::backup::Backup::new(source, &mut destination)
                .map_err(Self::projection_backup_error)?;
            loop {
                match backup.step(256).map_err(Self::projection_backup_error)? {
                    rusqlite::backup::StepResult::Done => break,
                    rusqlite::backup::StepResult::More => {}
                    rusqlite::backup::StepResult::Busy | rusqlite::backup::StepResult::Locked => {
                        return Err(Error::from_reason(
                            "PROJECTION_REBUILD_BUSY: projection publication could not acquire SQLite locks",
                        ));
                    }
                    _ => {
                        return Err(Error::from_reason(
                            "PROJECTION_REBUILD_FAILED: SQLite backup returned an unsupported step state",
                        ));
                    }
                }
            }
        }
        // Backup commits below SQLite's SQL executor and resets the schema.
        // Its pending WAL hook would otherwise checkpoint the published pages
        // during schema loading in the first query's prepare. Complete that
        // maintenance here, after Backup::drop releases the destination handle.
        // PASSIVE preserves pinned reader snapshots and accepts partial progress.
        destination
            .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
            .map_err(|error| {
                Error::from_reason(format!(
                    "PROJECTION_CHECKPOINT_FAILED: projection publication committed; {error}"
                ))
            })
'''
assert before.count(old)==1
candidate=before.replace(old,new)
(r/'proposed-publication-lib.rs').write_text(candidate,newline='\n')
(r/'proposed-publication.patch').write_text(''.join(difflib.unified_diff(before.splitlines(True),candidate.splitlines(True),fromfile='a/src/lib.rs',tofile='b/src/lib.rs')))
(r/'proposal-status.json').write_text(json.dumps({'status':'not_applied_pending_runtime_reproducer_and_RED','base_lib_sha256':hashlib.sha256((r/'entry-lib.rs').read_bytes()).hexdigest(),'candidate_lib_sha256':hashlib.sha256((r/'proposed-publication-lib.rs').read_bytes()).hexdigest(),'risk':'MEDIUM; publication maintenance and explicit post-commit error; no persistence format or API signature changes'},indent=2)+'\n')
