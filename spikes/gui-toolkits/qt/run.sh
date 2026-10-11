#!/bin/sh
# Runs the QML app with the Qt runner; arguments after -- reach the app.
export QML_XHR_ALLOW_FILE_READ=1
exec qml6 --apptype widget "$(dirname "$0")/app.qml" -- "$@"
