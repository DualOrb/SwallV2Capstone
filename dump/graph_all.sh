#!/bin/bash
FILES=/root/persistent/swall_v2/dump/dot/*

#Move each graph currently in the graphs directory to the "old" folder
mv graphs/* old_graphs/

#For each file in dot, graphify it and put it in graphs
for f in $FILES
do
    dot -Tsvg $f > "$f.svg"
done

#Cleanup unnecessary files and move svgs to graphs
rm dot/*.dot
mv dot/* graphs/


#TO Make use of graphify, do the following commands
# 1. apt-get install graphiz
# 2. export GST_DEBUG_DOT_DIR=/root/persistent/swall_v2/dump/
# 3. Run any GStreamer command/pipeline
# 4. Bash graph_all.sh
# 5. Open the generated graphs in graphs folder in a web browser or VS Code Extension
mv dot/* graphs/
