# Thoughts
Just some general thoughts about different components of the application


## Networking
### State Management
Keep track of a connection status: 
- `InitialisingConnection`, `InitialConnectionFailed` - these will show a UI displaying 'Connecting to server'
- `Connected`, `Disconnected`
Whilst the connection status is `InitialisingConnection` or `InitialConnectionFailed`, we will continue to try to connect to the server.


Once the connection is `Connected`, the state will be loaded and will load the UI