import { useState } from "react";
import carletonLogo from "./assets/CarletonUniversityLogo.png";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import BusStop from "./BusStop";


function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  let test_data = { "GetRouteSummaryForStopResult": { "StopNo": "5813", "Error": "", "StopDescription": "CARLETON U", "Routes": { "Route": [{ "RouteNo": "2", "RouteHeading": "South keys", "DirectionID": 1, "Direction": "", "Trips": [{ "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "South keys", "TripStartTime": "17:07", "AdjustedScheduleTime": "19", "AdjustmentAge": "0.55", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "South Keys", "TripStartTime": "17: 36", "AdjustedScheduleTime": "29", "AdjustmentAge": " - 1", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "South Keys", "TripStartTime": "17: 37", "AdjustedScheduleTime": "44", "AdjustmentAge": " - 1", "LastTripOfSchedule": false, "BusType": "" }] }, { "RouteNo": "7", "RouteHeading": "St - Laurent", "DirectionID": 1, "Direction": "", "Trips": [{ "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "St - Laurent", "TripStartTime": "17: 23", "AdjustedScheduleTime": "17", "AdjustmentAge": "0.72", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "St - Laurent", "TripStartTime": "17: 38", "AdjustedScheduleTime": "31", "AdjustmentAge": " - 1", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "St - Laurent", "TripStartTime": "18:07", "AdjustedScheduleTime": "60", "AdjustmentAge": " - 1", "LastTripOfSchedule": false, "BusType": "" }] }, { "RouteNo": "10", "RouteHeading": "Hurdman", "DirectionID": 0, "Direction": "", "Trips": [{ "Longitude": " - 75.70251528422038", "Latitude": "45.405879974365234", "GPSSpeed": "1", "TripDestination": "Hurdman", "TripStartTime": "16: 55", "AdjustedScheduleTime": "10", "AdjustmentAge": "0.65", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "Hurdman", "TripStartTime": "17: 11", "AdjustedScheduleTime": "25", "AdjustmentAge": "0.63", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "Hurdman", "TripStartTime": "17: 27", "AdjustedScheduleTime": "40", "AdjustmentAge": " - 1", "LastTripOfSchedule": false, "BusType": "" }] }, { "RouteNo": "111", "RouteHeading": "Baseline", "DirectionID": 1, "Direction": "", "Trips": [{ "Longitude": " - 75.6880874633789", "Latitude": "45.38251876831055", "GPSSpeed": "39", "TripDestination": "Baseline", "TripStartTime": "16: 58", "AdjustedScheduleTime": "7", "AdjustmentAge": "0.50", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "Baseline", "TripStartTime": "17: 13", "AdjustedScheduleTime": "14", "AdjustmentAge": "0.58", "LastTripOfSchedule": false, "BusType": "" }, { "Longitude": "", "Latitude": "", "GPSSpeed": "", "TripDestination": "Baseline", "TripStartTime": "17: 27", "AdjustedScheduleTime": "21", "AdjustmentAge": " - 1", "LastTripOfSchedule": false, "BusType": "" }] }] } } }

  const date = new Date();
  const currTime = date.getHours() + ':' + (date.getMinutes() < 10 ? '0' : '') + date.getMinutes()

  async function getSchedule() {
    return await JSON.parse(invoke('get_schedule_data'))
  }

  return (
    <div className="container">
      <div className="title-bar">
        <img src={carletonLogo} className="logo react" alt="React logo" />
        <h1>Bus Schedule</h1>
        <h1 className="current-time">{currTime}</h1>
      </div>

      <div className="schedule-container">
        <BusStop stopData={test_data["GetRouteSummaryForStopResult"]} />
      </div>
      
    </div>
  );
}

export default App;

