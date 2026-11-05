from pathlib import Path
r=Path(__file__).parent;p=r/'guard.ps1';s=p.read_text()
s=s.replace("$functionalOnly=($c.kind -eq 'functional_test' -and $c.cap_seconds -le 300", "$functionalOnly=($c.kind -eq 'functional_test' -and $null -ne $c.cap_seconds -and [int]$c.cap_seconds -gt 0 -and [int]$c.cap_seconds -le 300")
s=s.replace('$excluded[$id]=$w;continue', "$excluded[$id]=[pscustomobject]@{ProcessId=$w.ProcessId;ParentProcessId=$w.ParentProcessId;CreationDate=$w.CreationDate;Name=$w.Name;ExecutablePath=$w.ExecutablePath;parent_creation=$known[$parent].CreationDate;parent_name=$known[$parent].Name;role='non-compiling toolchain telemetry/PDB service'};continue")
s=s.replace('excluded_build_helpers=@($excluded.Values|Select-Object ProcessId,CreationDate,Name)', "excluded_build_helpers=@($excluded.Values);helper_finished_parent_condition=($cleanupComplete -and $p.HasExited -and $code -eq 0);helper_scope='Helpers are not terminated; successful Cargo completion and stopped tracked compiler clients are required before private-source edits. No shared source/target mutation.'")
p.write_text(s,newline='\n')
